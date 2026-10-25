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


if __name__ == '__main__':
    unittest.main()
