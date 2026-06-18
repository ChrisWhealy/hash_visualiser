import init, { run } from '/pkg/hash_visualiser.js';

// The server watches the .hv file and pushes its source over a Server-Sent Events stream, so the diagram
// re-renders live on every edit — no polling and no traffic between edits (and no wasm rebuild or restart).
const EVENTS_URL = '/events';

const app = document.getElementById('app');
const transport = document.getElementById('transport');
const banner = document.getElementById('error-banner');
const description = document.getElementById('description');

let lastSource = null;

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

await init();

// EventSource delivers the current source on connect, then a fresh message on every file change; it also
// reconnects automatically if the connection drops.
const events = new EventSource(EVENTS_URL);
events.onmessage = (e) => render(e.data);
events.onerror = () => console.warn('live-reload stream interrupted; reconnecting…');
