#!/bin/sh
# Assemble the public site into site/dist: the showcase files it loads plus the site layer. Deploy: npx wrangler deploy (from site/).
set -eu
cd "$(dirname "$0")"
rm -rf dist && mkdir dist
for f in lab.css tokens.js logs.js lab.js components.js fixtures.js showcase.js bboxy.png; do cp "../lab/web/$f" dist/; done
cp index.html site.css site.js og.png dist/
