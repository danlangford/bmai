BMAIR for the web
=================

This folder is a complete static website. It runs BMAIR, the Button Men AI,
inside the visitor's browser tab. Nothing is sent to a server, and the site
needs no build step, database, or server-side code.

Deploy
------

Upload every file and folder to any static host: GitHub Pages, Netlify,
Cloudflare Pages, Amazon S3, or a plain web server. Links are relative, so the
site also works from a subdirectory such as /bmair/.

Hosts should serve bmair.wasm as application/wasm. Most already do. If one
does not, the page still works and only compiles the engine more slowly.

Browsers refuse to start the engine from a file:// page. To try the site on
your own computer, serve this folder and open http://localhost:8000/:

    python3 -m http.server 8000

Browsers
--------

Current Chrome, Edge, Firefox, and Safari. The page needs module workers and
WebAssembly reference types, which arrived in Chrome and Edge 96, Firefox 114,
and Safari 15.

Updating
--------

Each build keeps its scripts and engine in a folder named for their
contents, so cached files from an earlier upload never mix with new ones.
index.html is the only file that changes in place:

1. Upload the new app-* folder.
2. Upload index.html last, so it never points at a folder that isn't there.
3. Keep the previous app-* folder until the next upload. It is under 1 MB,
   and visitors with the old page cached still need it.

Netlify, Cloudflare Pages, and GitHub Pages tell browsers to check
index.html for changes. Amazon S3 and plain web servers usually don't, and
browsers then guess how long to keep it, sometimes for days. On those hosts,
serve index.html with "Cache-Control: no-cache": object metadata on S3, or
add_header in nginx.

Files
-----

index.html              the page
app-*/                  its scripts, styles, examples, and the engine
app-*/bmair.wasm        the engine, the same file as the wasm32-wasip1 release
build-info.txt          the version and commit the site was built from
LICENSE.txt             the MIT license
