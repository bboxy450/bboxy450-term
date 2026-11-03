// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Web-only design iteration through Boxybook's isolated capture pipeline.
import fs from 'node:fs/promises';
import path from 'node:path';
import vm from 'node:vm';
import {spawnSync} from 'node:child_process';
const out=path.resolve(process.argv[2]||'target/startup-web');
const context={};context.window=context;
vm.createContext(context);
vm.runInContext(await fs.readFile('lab/web/startup.js','utf8'),context,{timeout:1000});
const cases=[];
for(const state of Object.keys(context.STARTUP_STATES))for(const theme of ['dark','light'])cases.push({
  component:'startup',state,slug:state.replaceAll(' ','-'),density:'cozy',theme,
  page:'startup.html',selector:'.card > .body',width:1246,height:800,checker:false,
  fixture:{layer:'components'},
});
await fs.mkdir(out,{recursive:true});
const manifest=path.join(out,'requested.json');
await fs.writeFile(manifest,JSON.stringify(cases,null,2));
const result=spawnSync(process.execPath,['crates/bboxybook/capture.mjs',out,manifest],{stdio:'inherit'});
if(result.error)throw result.error;
process.exitCode=result.status??1;
