"""Resource accounting must preserve workload behavior and honest attribution."""
import importlib.util
import json
import os
import select
import signal
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch, Mock

sys.dont_write_bytecode = True
SOURCE = Path(__file__).with_name('resource-ledger.py')
spec = importlib.util.spec_from_file_location('resource_ledger', SOURCE)
ledger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ledger)
spec = importlib.util.spec_from_file_location('inference_ledger', SOURCE.with_name('inference-ledger.py'))
inference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inference)


class LedgerContracts(unittest.TestCase):
    def test_unavailable_store_runs_command_once_and_preserves_exit_and_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            store = root / 'unavailable'
            store.write_text('not a directory')
            proof = root / 'calls'
            command = [sys.executable, '-c',
                'import pathlib,sys;pathlib.Path(sys.argv[1]).write_text("once");'
                'print("workload-output");raise SystemExit(7)', str(proof)]
            result = subprocess.run([sys.executable, '-I', str(SOURCE), 'run', 'unit',
                                     '--', *command], capture_output=True, text=True,
                                     env=dict(os.environ, BBOXY_LANDING_STATE_DIR=str(store)))
            self.assertEqual(result.returncode, 7)
            self.assertEqual(result.stdout, 'workload-output\n')
            self.assertEqual(proof.read_text(), 'once')
            self.assertIn('ledger: unavailable', result.stderr)

    def test_operation_counts_are_not_fabricated_bytes(self):
        usage = ledger.parse_time('1.20 real 0.35 user 0.04 sys\n'
                                  ' 1234 maximum resident set size\n'
                                  ' 21 block output operations\n')
        self.assertEqual(usage['cpu_user_seconds'], .35)
        self.assertEqual(usage['cpu_system_seconds'], .04)
        self.assertEqual(usage['peak_rss_bytes'], 1234)
        self.assertEqual(usage['block_output_operations'], 21)
        self.assertNotIn('disk_bytes_written', usage)

    def test_cache_binding_rejects_missing_reused_and_conflicting_owners(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(
                os.environ, BBOXY_LANDING_STATE_DIR=directory):
            root = (Path(directory) / 'worktree').resolve()
            bindings = Path(directory) / 'ledger-cache-bindings/4800'
            bindings.mkdir(parents=True)
            rows = {41: dict(start=100)}
            self.assertIsNone(ledger.cache_root(root, '4800', rows))
            value = dict(pid=41, start=99, worktree=str(root), port=4800, basis='fixture')
            (bindings / 'a.json').write_text(json.dumps(value))
            self.assertIsNone(ledger.cache_root(root, '4800', rows), 'PID reuse is unknown')
            value['start'] = 100
            (bindings / 'a.json').write_text(json.dumps(value))
            self.assertEqual(ledger.cache_root(root, '4800', rows)['pid'], 41)
            value['worktree'] = str((Path(directory) / 'another-worktree').resolve())
            (bindings / 'b.json').write_text(json.dumps(value))
            self.assertIsNone(ledger.cache_root(root, '4800', rows), 'shared port is unknown')

    def test_cache_children_join_only_a_live_recorded_stage_without_double_charging(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(
                os.environ, BBOXY_LANDING_STATE_DIR=directory):
            root = (Path(directory) / 'worktree').resolve()
            bindings = Path(directory) / 'ledger-cache-bindings/4800'
            bindings.mkdir(parents=True)
            value = dict(pid=8, start=80, worktree=str(root), port=4800, basis='fixture')
            (bindings / 'a.json').write_text(json.dumps(value))
            rows = {p: dict(pid=p, parent=parent, pgid=p, start=p*10,
                            user=2., system=0., written=0, rss=100)
                    for p,parent in [(1,0),(8,0),(9,8)]}
            previous = {(p,v['start']):dict(v,user=1.) for p,v in rows.items()}
            registration = dict(id='command',agent='fixture',cut='cut',stage='build',
                                pid=1,start=10,cache_port='4800',worktree=str(root))
            with patch.object(ledger, 'processes', return_value=rows):
                groups,_ = ledger.sample([registration],previous)
            self.assertEqual(len(groups),1)
            self.assertEqual(groups[0]['pids'],[1,8,9])
            self.assertEqual(groups[0]['cpu_user_seconds'],3.)
            rows.pop(1)
            with patch.object(ledger, 'processes', return_value=rows):
                groups,_ = ledger.sample([registration],previous)
            self.assertEqual(groups[0]['agent'],'unattributed')

    def test_cache_totals_include_already_reaped_compiler_children(self):
        usage = ledger.Usage()
        usage.start = 100
        usage.user, usage.child_user = 24_000_000, 48_000_000
        usage.system, usage.child_system = 12_000_000, 24_000_000
        def fill(pid, flavor, pointer):
            ledger.ctypes.memmove(pointer, ledger.ctypes.byref(usage), ledger.ctypes.sizeof(usage))
            return 0
        library = Mock()
        library.proc_pid_rusage.side_effect = fill
        with patch.dict(os.environ, RUSTC_WRAPPER='sccache'), \
             patch.object(ledger, 'cache_root', return_value=dict(pid=41,start=100)), \
             patch.object(ledger.ctypes, 'CDLL', return_value=library), \
             patch.object(ledger, 'mach_seconds_per_tick', return_value=1/24_000_000):
            totals = ledger.cache_totals(Path.cwd())
        self.assertEqual(totals['user'],3.)
        self.assertEqual(totals['system'],1.5)

    @unittest.skipUnless(sys.platform == 'darwin', 'time -l accounting pipe')
    def test_stage_keeps_waited_and_cache_cpu_separate_and_adds_each_once(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(
                os.environ, BBOXY_LANDING_STATE_DIR=directory), \
             patch.object(ledger, 'start_observer', return_value=os.getpid()), \
             patch.object(ledger, 'cache_totals', side_effect=[
                 dict(pid=41,start=100,user=10.,system=20.),
                 dict(pid=41,start=100,user=12.,system=23.)]):
            stage = ledger.Stage('build',Path(directory))
            os.write(stage.writer,b'0.00 real 1.00 user 2.00 sys\n123 maximum resident set size\n')
            stage.attach(os.getpid())
            stage.finish(0)
            path = next(Path(directory).glob('ledger/*/ledger.jsonl'))
            row = json.loads(path.read_text())
            self.assertEqual(row['waited_cpu_user_seconds'],1.)
            self.assertEqual(row['cache_cpu_user_seconds'],2.)
            self.assertEqual(row['cpu_user_seconds'],3.)
            self.assertEqual(row['cpu_system_seconds'],5.)

    def test_deepest_recorded_root_owns_child_once_and_pid_reuse_is_unattributed(self):
        rows = {p: dict(pid=p, parent=parent, pgid=1, start=start,
                        user=2., system=.5, written=32, rss=100)
                for p,parent,start in [(1,0,10),(2,1,20),(3,2,30),(4,1,41)]}
        registrations = [dict(id=str(p),pid=p,start=start,agent='agent',
                              cut='cut',stage=stage)
                         for p,start,stage in [(1,10,'integration'),(2,20,'unit'),(4,40,'build')]]
        previous = {(p,r['start']): dict(r,user=1.,system=0.,written=16)
                    for p,r in rows.items()}
        with patch.object(ledger, 'processes', return_value=rows):
            groups, _ = ledger.sample(registrations, previous)
        by_id = {g['id']:g for g in groups}
        self.assertEqual(by_id['2']['pids'], [2,3])
        self.assertEqual(by_id['2']['cpu_user_seconds'], 2.)
        # PID4's old registration cannot claim a reused process; its live parent
        # may still account it as a real descendant of PID1.
        self.assertNotIn('4',by_id)
        self.assertEqual(by_id['1']['pids'], [1,4])
        self.assertEqual(sum(g['cpu_user_seconds'] for g in groups), 4.)
        with patch.object(ledger, 'processes', return_value={4:rows[4]}):
            groups,_ = ledger.sample([registrations[-1]], previous)
        self.assertEqual(groups[0]['agent'],'unattributed')

    @unittest.skipUnless(sys.platform == 'darwin', 'Mach CPU accounting')
    def test_libproc_seconds_are_bracketed_by_self_rusage(self):
        import resource
        before = resource.getrusage(resource.RUSAGE_SELF)
        row = ledger.processes()[os.getpid()]
        after = resource.getrusage(resource.RUSAGE_SELF)
        measured = row['user'] + row['system']
        # getrusage's timeval is quantized to microseconds. This is a units
        # cross-check between two accounting APIs, not a wall-clock budget.
        self.assertGreaterEqual(measured + .000001, before.ru_utime + before.ru_stime)
        self.assertLessEqual(measured - .000001, after.ru_utime + after.ru_stime)

    @unittest.skipUnless(sys.platform == 'darwin', 'POSIX signal ownership')
    def test_signal_reaches_only_the_recorded_command_tree(self):
        with tempfile.TemporaryDirectory() as directory:
            child = subprocess.Popen([sys.executable,'-I',str(SOURCE),'run','unit','--',
                sys.executable,'-c',
                'import os,sys;print(os.getpid(),flush=True);sys.stdin.read()'],
                stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                text=True,env=dict(os.environ,BBOXY_LANDING_STATE_DIR=directory))
            owned = None
            try:
                self.assertTrue(select.select([child.stdout],[],[],30)[0], 'command never started')
                owned = int(child.stdout.readline())
                child.send_signal(signal.SIGTERM)
                child.wait(timeout=30)
                child.communicate()
                self.assertEqual(child.returncode,143)
                with self.assertRaises(ProcessLookupError):
                    os.kill(owned,0)
                owned = None
            finally:
                if owned:
                    try: os.kill(owned,signal.SIGTERM)
                    except ProcessLookupError: pass
                if child.poll() is None:
                    child.kill();child.communicate()

    @unittest.skipUnless(sys.platform == 'darwin', 'time -l is the macOS accounting source')
    def test_real_failed_child_has_rusage_and_private_append_only_record(self):
        with tempfile.TemporaryDirectory() as directory:
            env = dict(os.environ,BBOXY_LANDING_STATE_DIR=directory,
                       BBOXY_LEDGER_AGENT='fixture')
            result = subprocess.run([sys.executable,'-I',str(SOURCE),'run','unit','--',
                                     sys.executable,'-c','raise SystemExit(9)'],env=env,
                                     capture_output=True,text=True)
            self.assertEqual(result.returncode,9)
            path = next(Path(directory).glob('ledger/*/ledger.jsonl'))
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            row = next(r for r in rows if r['kind']=='stage')
            self.assertEqual(row['exit'],9)
            self.assertEqual(row['agent'],'fixture')
            self.assertIsInstance(row['cpu_user_seconds'],float)
            self.assertGreater(row['peak_rss_bytes'],0)
            self.assertEqual(path.stat().st_mode & 0o777,0o600)
            self.assertEqual(list((Path(directory)/'ledger-processes').glob('*.json')),[])

    def test_smoke_directory_retains_the_checked_cut_from_its_driver(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            env = dict(os.environ,BBOXY_LANDING_STATE_DIR=str(root/'state'),
                       BBOXY_LEDGER_AGENT='fixture',BBOXY_LEDGER_BRANCH='fixture-cut',
                       BBOXY_LEDGER_SHA='a'*40)
            result = subprocess.run([sys.executable,'-I',str(SOURCE.resolve()),'run','smoke','--',
                                     sys.executable,'-c','pass'],cwd=root,env=env,
                                     capture_output=True,text=True)
            self.assertEqual(result.returncode,0)
            path = next((root/'state').glob('ledger/*/ledger.jsonl'))
            row = next(json.loads(line) for line in path.read_text().splitlines()
                       if json.loads(line)['kind']=='stage')
            self.assertEqual(row['cut'],dict(branch='fixture-cut',sha='a'*40))
            self.assertEqual(row['agent'],'fixture')


class InferenceContracts(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.project = self.root / 'bboxy450-term'
        self.codex = self.root / 'codex'
        self.claude = self.root / 'claude'
        self.codex.mkdir()
        self.claude.mkdir()
        self.state = self.root / 'state'
        environment = patch.dict(os.environ, BBOXY_LANDING_STATE_DIR=str(self.state))
        environment.start()
        self.addCleanup(environment.stop)

    def record(self, path, kind, second, **fields):
        row = dict(type=kind, timestamp=f'2026-10-25T12:00:{second:02}.000Z', **fields)
        with path.open('a') as stream:
            stream.write(json.dumps(row) + '\n')

    def collect(self):
        return inference.collect(self.codex, self.claude, self.project)

    def rows(self):
        return [json.loads(line) for path in (self.state/'ledger').glob('*/ledger.jsonl')
                for line in path.read_text().splitlines() if json.loads(line).get('stage')=='inference']

    def codex_start(self, path):
        self.record(path, 'session_meta', 0, payload=dict(id='fixture-codex',
                    cwd=str(self.project), git=dict(branch='stale-launch-cut', commit_hash='a'*40),
                    base_instructions='PRIVATE BODY MUST NOT BE SAVED'))

    def codex_usage(self, path, second, input, cached, output, reasoning):
        values = dict(input_tokens=input, cached_input_tokens=cached,
                      output_tokens=output, reasoning_output_tokens=reasoning)
        self.record(path, 'token_usage_record', second, payload=dict(thread_token_usage=values))
        self.record(path, 'event_msg', second, payload=dict(type='token_count',
                    info=dict(total_token_usage=values, last_token_usage=values)))

    def test_codex_mirrors_counter_deltas_and_private_incremental_restart(self):
        path = self.codex/'fixture.jsonl'
        self.codex_start(path)
        self.codex_usage(path, 2, 100, 80, 20, 15)
        self.collect()
        self.assertEqual(self.rows()[0]['tokens'],dict(zip(inference.TOKENS,(80,20,15,5))))
        self.assertEqual(self.rows()[0]['cut'],dict(branch=None,sha=None))
        self.assertEqual(self.collect()['rows'],0)
        self.codex_usage(path, 4, 180, 90, 60, 35)
        self.collect()
        self.assertEqual(self.rows()[1]['tokens'],dict(zip(inference.TOKENS,(10,70,20,20))))
        for saved in (self.state/'ledger').rglob('*'):
            if saved.is_file():
                self.assertEqual(saved.stat().st_mode & 0o777,0o600)
                self.assertNotIn('PRIVATE BODY',saved.read_text())
                self.assertNotIn('stale-launch-cut',saved.read_text())

    def test_claude_streamed_usage_is_not_multiplied_and_creation_is_uncached(self):
        path=self.claude/'fixture.jsonl'
        self.record(path,'api-request',0,id='request',sessionId='fixture-claude')
        values=dict(input_tokens=3,cache_creation_input_tokens=17,cache_read_input_tokens=80,
                    output_tokens=20,output_tokens_details=dict(thinking_tokens=15))
        for second in (2,3,4):
            self.record(path,'assistant',second,cwd=str(self.project),gitBranch='reported-cut',
                sessionId='fixture-claude',requestRef='request',message=dict(id='msg',usage=values,
                content=[dict(type='text',text='PRIVATE BODY input_tokens 900000')]))
        self.collect()
        row=self.rows()[0]
        self.assertEqual(row['tokens'],dict(zip(inference.TOKENS,(80,20,15,5))))
        self.assertEqual(row['cut'],dict(branch='reported-cut',sha=None))
        self.assertEqual(row['wall_seconds'],4.)
        self.assertEqual(self.collect()['rows'],0)
        values['output_tokens']=30
        self.record(path,'assistant',5,cwd=str(self.project),sessionId='fixture-claude',
            requestRef='request',message=dict(id='msg',usage=values))
        self.collect()
        self.assertEqual(self.rows()[1]['tokens']['output_visible_tokens'],10)
        self.assertEqual(self.rows()[1]['wall_seconds'],1.)

    def test_codex_display_counters_do_not_override_the_request_ledger(self):
        path=self.codex/'fixture.jsonl'
        self.codex_start(path)
        self.codex_usage(path,2,100,80,20,15)
        self.record(path,'event_msg',3,payload=dict(type='token_count',info=dict(total_token_usage=dict(
            input_tokens=110,cached_input_tokens=90,output_tokens=25,reasoning_output_tokens=15))))
        self.codex_usage(path,4,105,82,22,16)
        result=self.collect()
        self.assertNotIn('codex_counter_regression',result['issues'])
        self.assertEqual(self.rows()[0]['tokens'],dict(zip(inference.TOKENS,(82,23,16,6))))

    def test_late_thinking_usage_revises_visible_output_without_overcount(self):
        path=self.claude/'fixture.jsonl'
        values=dict(input_tokens=3,cache_creation_input_tokens=0,cache_read_input_tokens=7,
                    output_tokens=20,output_tokens_details=dict(thinking_tokens=5))
        self.record(path,'assistant',1,cwd=str(self.project),sessionId='fixture',message=dict(id='msg',usage=values))
        self.collect()
        values['output_tokens_details']['thinking_tokens']=10
        self.record(path,'assistant',2,cwd=str(self.project),sessionId='fixture',message=dict(id='msg',usage=values))
        self.collect()
        self.assertEqual(self.rows()[-1]['tokens']['output_visible_tokens'],-5)
        self.assertEqual(sum(r['tokens']['output_visible_tokens'] for r in self.rows()),10)
        self.assertEqual(sum(r['tokens']['output_reasoning_tokens'] for r in self.rows()),10)

    def test_missing_reasoning_metadata_is_unknown_and_another_project_is_excluded(self):
        path=self.claude/'fixture.jsonl'
        for second,cwd,msg in [(0,self.root/'different-repo','other'),(1,self.project,'ours')]:
            self.record(path,'assistant',second,cwd=str(cwd),sessionId='fixture',message=dict(id=msg,
                usage=dict(input_tokens=3,cache_creation_input_tokens=0,cache_read_input_tokens=7,output_tokens=4)))
        self.collect()
        row=self.rows()[0]
        self.assertEqual(row['tokens']['input_uncached_tokens'],3)
        self.assertIsNone(row['tokens']['output_reasoning_tokens'])
        self.assertIsNone(row['tokens']['output_visible_tokens'])
        self.assertIsNone(row['wall_seconds'])

    def test_open_tool_ids_and_idle_gaps_are_not_inference_wall(self):
        path=self.codex/'fixture.jsonl'
        self.codex_start(path)
        self.record(path,'response_item',2,payload=dict(type='function_call',call_id='a',arguments='PRIVATE'))
        self.record(path,'response_item',50,payload=dict(type='function_call_output',call_id='a',output='PRIVATE'))
        self.codex_usage(path,55,10,0,2,1)
        self.record(path,'event_msg',56,payload=dict(type='task_complete'))
        with path.open('a') as stream:
            stream.write(json.dumps(dict(type='event_msg',timestamp='2026-10-25T13:00:00Z',payload=dict(type='task_started')))+'\n')
        self.collect()
        self.assertEqual(self.rows()[0]['wall_seconds'],8.)

    def test_trailing_partial_record_then_replay_of_a_rotated_file_counts_once(self):
        path=self.codex/'fixture.jsonl'
        self.codex_start(path)
        self.codex_usage(path,2,10,5,4,2)
        with path.open('a') as stream:stream.write('{"type":"event_msg"')
        self.collect()
        self.assertEqual(self.collect()['rows'],0)
        with path.open('a') as stream:stream.write(',"timestamp":"2026-10-25T12:00:03Z","payload":{"type":"task_complete"}}\n')
        self.collect()
        wall=sum(row['wall_seconds'] or 0 for row in self.rows())
        old=path.read_bytes()
        path.unlink()
        path.write_bytes(old)
        self.collect()
        self.assertEqual(sum(row['tokens']['input_cached_tokens'] or 0 for row in self.rows()),5)
        self.assertEqual(sum(row['wall_seconds'] or 0 for row in self.rows()),wall)

    def test_wall_is_split_at_midnight_and_an_explicit_cut_boundary(self):
        progress=dict(files={},usage={})
        binding=dict(harness='claude',session='fixture',agent='agent',
                     start='2026-10-26T00:00:01Z',end='2026-10-26T01:00:00Z',branch='cut',sha='b'*40)
        reader=inference.Reader(progress,self.project,[binding])
        reader.wall('claude',dict(session='fixture',cwd=str(self.project)),
                    inference.stamp('2026-10-25T23:59:59Z'),inference.stamp('2026-10-26T00:00:03Z'))
        rows=list(reader.rows.values())
        self.assertEqual(len(rows),3)
        self.assertEqual(sorted(r['wall_seconds'] for r in rows),[1.,1.,2.])
        self.assertEqual(next(r for r in rows if r['agent']=='agent')['wall_seconds'],2.)

    def test_explicit_bounded_attribution_and_overlapping_bindings_remain_unknown(self):
        directory=self.state/'ledger'
        directory.mkdir(parents=True)
        binding=dict(harness='codex',session='fixture-codex',agent='fixture-agent',
                     start='2026-10-25T12:00:00Z',end='2026-10-25T12:00:03Z',branch='fixture-cut',sha='b'*40)
        (directory/'inference-bindings.json').write_text(json.dumps([binding]))
        path=self.codex/'fixture.jsonl'
        self.codex_start(path)
        self.codex_usage(path,2,10,5,4,2)
        self.collect()
        self.assertEqual(self.rows()[0]['agent'],'fixture-agent')
        self.assertEqual(self.rows()[0]['cut'],dict(branch='fixture-cut',sha='b'*40))
        binding['end']='2026-10-25T12:00:10Z'
        (directory/'inference-bindings.json').write_text(json.dumps([binding,binding]))
        self.codex_usage(path,4,20,10,8,4)
        result=self.collect()
        self.assertGreater(result['issues']['ambiguous_binding'],0)
        self.assertEqual(self.rows()[-1]['agent'],'unattributed')

    def test_interrupted_export_is_idempotent_before_checkpoint_advance(self):
        path=self.codex/'fixture.jsonl'
        self.codex_start(path)
        self.codex_usage(path,2,10,5,4,2)
        append=inference.ledger.append
        def wrote_then_crashed(row):
            append(row)
            raise OSError('fixture interruption after append')
        with patch.object(inference.ledger,'append',side_effect=wrote_then_crashed):
            with self.assertRaises(OSError):self.collect()
        self.assertEqual(len(self.rows()),1)
        self.assertEqual(self.collect()['rows'],0)
        self.assertEqual(len(self.rows()),1)

    def test_overlapping_reader_skips_without_waiting_and_unavailable_store_is_observational(self):
        import fcntl
        directory=self.state/'ledger'
        directory.mkdir(parents=True)
        with (directory/'inference.lock').open('w') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.assertEqual(self.collect()['skipped'],'reader already active')
        unavailable=self.root/'unavailable'
        unavailable.write_text('file')
        result=subprocess.run([sys.executable,'-I',str(SOURCE.with_name('inference-ledger.py')),
            '--codex-root',str(self.codex),'--claude-root',str(self.claude),'--project',str(self.project)],
            env=dict(os.environ,BBOXY_LANDING_STATE_DIR=str(unavailable)),capture_output=True,text=True)
        self.assertEqual(result.returncode,0)
        self.assertIn('inference unavailable (NotADirectoryError)',result.stderr)
        self.assertNotIn('Traceback',result.stderr)

    def test_night_collects_usage_without_changing_a_failed_night_exit(self):
        scripts=self.root/'scripts'
        scripts.mkdir()
        (scripts/'e2e-night.sh').write_text(SOURCE.with_name('e2e-night.sh').read_text())
        (scripts/'inference-ledger.py').write_text('print("fixture-metrics");raise SystemExit(9)\n')
        (scripts/'landing_recovery.py').write_text('import sys;print("fixture-night");raise SystemExit(7)\n')
        result=subprocess.run(['sh',str(scripts/'e2e-night.sh'),'--all'],capture_output=True,text=True)
        self.assertEqual(result.returncode,7)
        self.assertEqual(result.stdout,'fixture-night\n')
        self.assertEqual(result.stderr,'fixture-metrics\n')


if __name__ == '__main__':
    unittest.main()
