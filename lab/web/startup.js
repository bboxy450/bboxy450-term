// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Public observations from cockpit/startup.rs tests, without accounts or secrets.
(() => {
  const normal = {title: 'Opening your cockpit', description: 'Your sessions appear when each check is complete.', elapsed_secs: 1, active_step: 0, history: null, details: [], error: null, logs: []};
  const history = {stream: 'account', verified: 25, total: 100};
  const failed = {...normal, title: 'Startup needs attention', description: 'The highlighted step could not finish. Check the details and retry.', elapsed_secs: 5, active_step: 2, history, error: 'History signature failed', logs: [['App log', '/tmp/bboxy-startup/app.log'], ['Runtime log', '/tmp/bboxy-startup/logic.log']]};
  window.STARTUP_STATES = {
    starting: normal,
    'existing supervisor': {...normal, details: [['Existing supervisor', 'Process 42']]},
    'starting runtime': {...normal, active_step: 1},
    'verifying history': {...normal, elapsed_secs: 3, active_step: 2, history},
    'empty history': {...normal, active_step: 2, history: {...history, verified: 0, total: 0}},
    attaching: {...normal, active_step: 3},
    reconnecting: {...normal, title: 'Reconnecting your cockpit', description: 'Restoring the connection before the cockpit opens.', elapsed_secs: 2, details: [['Reconnect attempt 2', 'GUI connection closed']]},
    'refreshing runtime': {...normal, title: 'Refreshing session runtime', description: 'Updating the runtime before reconnecting your cockpit.', active_step: 1, details: [['Reconnect attempt 2', 'GUI connection closed'], ['Runtime refresh', 'schema changed']]},
    failed,
    'long failure': {...failed, error: 'History signature failed while verifying a retained account stream. Startup is paused until the signed history can be verified. The records and the diagnostic paths below remain available for inspection.', logs: [['App log', '/tmp/bboxy-startup/private-account-with-a-long-descriptive-directory-name/run/app.log'], ['Runtime log', '/tmp/bboxy-startup/private-account-with-a-long-descriptive-directory-name/run/logic.log']]},
  };
  const steps = ['Connect to supervisor', 'Start session runtime', 'Verify signed history', 'Attach cockpit'];
  const text = (s, step='body', color, weight=400) => UI.Text(String(s), {step, color, weight});
  const stack = (dir, gap, ...children) => UI.Stack({dir, gap, align: dir === 'row' ? 'center' : 'stretch'}, ...children);
  const surface = (fill, padding, radius, ...children) => {
    const node = UI.Box('card', ...children);
    Object.assign(node.style, {display:'flex',flexDirection:'column',alignItems:'stretch',gap:'0px',padding:padding+'px',border:'0',borderRadius:radius+'px',background:fill,boxShadow:'none'});
    return node;
  };
  const tint = (color, alpha) => `color-mix(in srgb, ${color} ${alpha}%, transparent)`;
  window.startupModal = data => {
    const active = Math.min(data.active_step, 3), ink = data.error ? 'var(--c-errored)' : 'var(--accent)';
    const rows = steps.map((label, index) => {
      const current = index === active, done = index < active;
      const color = current ? ink : done ? 'var(--accent)' : 'var(--fg-dim)';
      const mark = surface(tint(color, 12), 0, 8, text(done ? '✓' : current && data.error ? '!' : index+1, 'body', color, 600));
      Object.assign(mark.style, {width:'28px',height:'28px',alignItems:'center',flexShrink:'0'});
      const status = done ? 'Done' : current && data.error ? 'Failed' : current ? 'In progress' : 'Pending';
      const header = stack('row', 12, mark, text(label, 'body', current || done ? 'var(--fg)' : 'var(--fg-dim)', current ? 600 : 400), UI.Spacer(), text(status, 'caption', color, current ? 600 : 400));
      header.lastElementChild.style.flexShrink = '0';
      const contents = [header];
      if (index === 2 && data.history) {
        const h = data.history;
        const progress = [text(`${h.verified} of ${h.total} records`, 'heading', 'var(--fg)', 600), text(`Stream · ${h.stream}`, 'caption', 'var(--fg-dim)')];
        if (h.total > 0) {
          const fraction = Math.max(0, Math.min(1, h.verified / h.total));
          const pieces=[];
          for (const [share, color] of [[fraction, ink], [1-fraction, 'transparent']]) if (share > 0) {
            const piece = surface(color, 0, 2);piece.style.flex = share;pieces.push(piece);
          }
          const track = surface('var(--surface-2)', 0, 2, ...pieces);
          Object.assign(track.style, {flexDirection:'row',height:'4px'});
          track.setAttribute('role', 'progressbar');track.setAttribute('aria-label', 'Verified history');
          track.setAttribute('aria-valuenow', Math.min(h.verified,h.total));track.setAttribute('aria-valuemax', h.total);
          progress.push(track);
        }
        contents.push(stack('column', 8, ...progress));
      }
      const row = surface(current ? tint(ink,6) : 'transparent', 8, 12, stack('column', 12, ...contents));
      row.setAttribute('role','listitem');row.setAttribute('aria-label',label+': '+status);
      if (current) row.setAttribute('aria-current','step');
      return row;
    });
    const checklist = stack('column', 4, ...rows);checklist.setAttribute('role','list');
    const body = [stack('column',8,text(data.title,'heading','var(--fg)',600),text(data.description,'body','var(--fg-dim)')),checklist];
    for (const [label,value] of data.details) body.push(stack('column',4,text(label,'caption','var(--fg-dim)'),text(value,'body','var(--fg)')));
    if (data.error) {const error = surface(tint('var(--c-errored)',6),12,8,text(data.error,'body','var(--fg)'));error.setAttribute('role','alert');body.push(error);}
    if (data.logs.length) body.push(stack('column',8,...data.logs.map(([label,path])=>stack('column',4,text(label,'caption','var(--fg-dim)',600),text(path,'caption','var(--fg-dim)')))));
    const quit = UI.Button('Quit',{kind:'ghost'});quit.onclick=()=>panel.replaceWith(Lab.el('p',{},'Preview closed. Select a startup state above to reopen.'));
    const footer=[text(`${data.elapsed_secs} s on this step`,'caption','var(--fg-dim)'),UI.Spacer(),quit];
    if (data.error) {const retry=UI.Button('Retry');retry.onclick=()=>panel.replaceWith(startupModal(STARTUP_STATES.starting));footer.push(retry);}
    body.push(stack('row',8,...footer));
    const panel=UI.Box('modal',stack('column',data.error ? 8 : 12,...body));
    Object.assign(panel.style,{display:'flex',flexDirection:'column',alignItems:'stretch',width:'560px',maxWidth:'100%',padding:'16px'});
    panel.classList.add('startup-card');panel.bboxyStartup=data;
    panel.setAttribute('role','dialog');panel.setAttribute('aria-label',data.title);
    return panel;
  };
})();
