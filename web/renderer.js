import init, { run } from '/pkg/hash_visualiser.js';

// The sidebar lists the .hv files (GET /api/files); selecting one fetches and renders it. The server also watches the
// hv/ folder and pushes each changed file over a Server-Sent Events stream, so the *currently selected* file re-renders
// live on every edit — no polling and no traffic between edits (and no wasm rebuild or restart).
const FILES_URL = '/api/files';
const EVENTS_URL = '/events';
const DEFAULT_FILE = 'sha3.hv';

const app = document.getElementById('app');
const transport = document.getElementById('transport');
const banner = document.getElementById('error-banner');
const description = document.getElementById('description');
const fileList = document.getElementById('file-list');

let currentFile = null;
let lastSource = null;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
function render(source) {
  if (source === lastSource) return; // unchanged since last render
  lastSource = source;

  // Clear the previous render before drawing afresh (run() appends new <svg> elements).
  app.innerHTML = '';
  transport.innerHTML = '';

  // A stale description may no longer match the edited source: hide it (and forget which node it showed).
  description.innerHTML = '';
  description.removeAttribute('data-shown');

  try {
    run('app', source);
    banner.style.display = 'none';
    banner.textContent = '';
  } catch (err) {
    banner.style.display = 'block';
    banner.textContent = `Error: ${err}`;
    console.error(err);
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
async function selectFile(name) {
  currentFile = name;

  for (const li of fileList.children) {
    li.classList.toggle('active', li.dataset.file === name);
  }

  lastSource = null; // force a fresh render even if the new file's bytes coincide with the old

  try {
    const source = await fetch(`/hv/${encodeURIComponent(name)}`, { cache: 'no-store' }).then((r) => r.text());
    render(source);
  } catch (err) {
    banner.style.display = 'block';
    banner.textContent = `Error loading ${name}: ${err}`;
    console.error(err);
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
async function loadFileList() {
  const files = await fetch(FILES_URL).then((r) => r.json());
  fileList.replaceChildren();

  for (const name of files) {
    const li = document.createElement('li');
    li.textContent = name;
    li.dataset.file = name;
    li.addEventListener('click', () => selectFile(name));
    fileList.appendChild(li);
  }

  return files;
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Entry point
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
await init();

const files = await loadFileList();

if (files.length) {
  await selectFile(files.includes(DEFAULT_FILE) ? DEFAULT_FILE : files[0]);
}

// EventSource pushes a { file, content } message whenever any .hv file changes; re-render only when the change is to the
// file we're currently showing. EventSource also reconnects automatically if the connection drops.
const events = new EventSource(EVENTS_URL);

events.onmessage = (e) => {
  const { file, content } = JSON.parse(e.data);

  if (file === currentFile) render(content);
};

events.onerror = () => console.warn('live-reload stream interrupted; reconnecting…');
