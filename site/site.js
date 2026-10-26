// Public site layer over the showcase: a Download action in the title bar and a launcher-style panel for it.
// On a phone the full-window mock does not fit, so the panel stands alone.
(function () {
  const VERSION = '0.1.0';
  const ZIP = 'bboxy450-term-' + VERSION + '-macos-arm64.zip';
  const REL = 'https://github.com/bboxy450/bboxy450-term-releases/releases/download/v' + VERSION + '/';
  const el = Lab.el;

  function copyRow(label, cmd) {
    const note = el('span', { class: 'note' }, 'copy');
    const row = el('button', { class: 'opt', type: 'button', onclick: () => {
      const done = () => { note.textContent = 'copied'; setTimeout(() => { note.textContent = 'copy'; }, 1600); };
      if (navigator.clipboard) navigator.clipboard.writeText(cmd).then(done, () => {}); else done();
    } }, el('span', { class: 'what' }, el('b', {}, label), el('code', {}, cmd)), note);
    return row;
  }

  function panel(standalone) {
    const close = el('button', { class: 'x', type: 'button', title: 'close (esc)', onclick: hide }, C.kbd('esc'));
    return el('div', { class: 'dl' + (standalone ? ' standalone' : ''), role: 'dialog', 'aria-label': 'Download bboxy450-term' },
      el('div', { class: 'head' }, C.logo(40), el('div', {}, el('b', {}, 'bboxy450-term ' + VERSION), el('small', {}, 'A GPU terminal for running agent swarms. macOS on Apple silicon.')), standalone ? null : close),
      el('a', { class: 'opt sel', href: REL + ZIP }, el('span', { class: 'what' }, el('b', {}, 'Download for macOS'), el('code', {}, ZIP)), C.kbd('↵')),
      copyRow('Homebrew', 'brew install --cask bboxy450/tap/bboxy450-term'),
      copyRow('Cargo', 'cargo install bboxy450-term'),
      el('p', { class: 'sig' }, 'Signed with GPG key 844ECC0587DE7AB0. ',
        el('a', { href: REL + ZIP + '.asc' }, 'Signature'), ' · ',
        el('a', { href: REL + ZIP + '.sha256' }, 'SHA-256'), ' · ',
        el('a', { href: 'https://crates.io/crates/bboxy450-term' }, 'crates.io'), ' · ',
        el('a', { href: 'https://github.com/bboxy450/bboxy450-term-releases' }, 'Releases')));
  }

  const scrim = el('div', { class: 'dl-scrim', hidden: '', onclick: e => { if (e.target === scrim) hide(); } });
  document.body.append(scrim);
  function show() { scrim.innerHTML = ''; scrim.append(panel(false)); scrim.hidden = false; scrim.querySelector('.opt.sel').focus(); }
  function hide() { scrim.hidden = true; if (location.hash === '#download') history.replaceState(null, '', location.pathname + location.search); }
  addEventListener('keydown', e => { if (!scrim.hidden && e.key === 'Escape') { e.stopPropagation(); hide(); } }, true);

  const bar = document.querySelector('.titlebar.app-bar');
  if (bar) bar.querySelector('.spacer').after(el('button', { class: 'dl-btn', type: 'button', onclick: show }, 'Download'));
  if (location.hash === '#download') show();

  // phones: no mock, just the panel
  const phone = matchMedia('(max-width: 760px)');
  const solo = el('div', { class: 'dl-solo' }, panel(true));
  document.body.append(solo);
})();
