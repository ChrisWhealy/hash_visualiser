import init, { run } from '/pkg/hash_visualiser.js'

// The sidebar lists the .hv files (GET /api/files); selecting one fetches and renders it. The server also watches the
// hv/ folder and pushes each changed file over a Server-Sent Events stream, so the *currently selected* file re-renders
// live on every edit — no polling and no traffic between edits (and no wasm rebuild or restart).
const FILES_URL = '/api/files'
const EVENTS_URL = '/events'
const DEFAULT_FILE = 'sha3.hv'

const app = document.getElementById('app')
const transport = document.getElementById('transport')
const banner = document.getElementById('error-banner')
const description = document.getElementById('description')
const fileList = document.getElementById('file-list')

let currentFile = null
let lastSource = null
let currentFiles = [] // most recent file list, so a folder toggle can re-render the tree
const collapsedFolders = new Set() // folder paths the user has collapsed

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const render = source => {
  // Bail out early if unchanged since last render
  if (source === lastSource) return
  
  lastSource = source

  // Clear the previous render before drawing afresh (run() appends new <svg> elements).
  app.innerHTML = ''
  transport.innerHTML = ''

  // A stale description may no longer match the edited source: hide it (and forget which node it showed).
  description.innerHTML = ''
  description.removeAttribute('data-shown')

  try {
    run('app', source)
    banner.style.display = 'none'
    banner.textContent = ''
  } catch (err) {
    banner.style.display = 'block'
    banner.textContent = `Error: ${err}`
    console.error(err)
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The URL for a file path: each segment is encoded but the slashes between subdirectories are kept.
const fileUrl = path => '/hv/' + path.split('/').map(encodeURIComponent).join('/')

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Mark the list entry for the currently-selected file active (and the rest inactive).
const highlightActive = () => {
  for (const el of fileList.querySelectorAll('[data-file]')) {
    el.classList.toggle('active', el.dataset.file === currentFile)
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const selectFile = async path => {
  currentFile = path
  highlightActive()

  lastSource = null // force a fresh render even if the new file's bytes coincide with the old

  try {
    const source = await fetch(fileUrl(path), { cache: 'no-store' }).then((r) => r.text())
    render(source)
  } catch (err) {
    banner.style.display = 'block'
    banner.textContent = `Error loading ${path}: ${err}`
    console.error(err)
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Group a flat list of relative paths (e.g. "binary_operations/01_add.hv") into a nested folder/file tree.
const buildTree = paths => {
  const root = { dirs: new Map(), files: [] }

  for (const path of paths) {
    const parts = path.split('/')
    let node = root

    for (const dir of parts.slice(0, -1)) {
      if (!node.dirs.has(dir)) node.dirs.set(dir, { dirs: new Map(), files: [] })
      node = node.dirs.get(dir)
    }

    node.files.push(path)
  }

  return root
}

// Render a tree node into `container`: folders (collapsible) first, then files. Paths are pre-sorted, so the Map and
// the file array are already in alphabetical order.
const appendTree = (node, prefix, container) => {
  for (const [dir, child] of node.dirs) {
    const folderPath = prefix ? `${prefix}/${dir}` : dir
    const collapsed = collapsedFolders.has(folderPath)

    const li = document.createElement('li')
    li.className = 'folder'

    const label = document.createElement('div')
    label.className = 'folder-label'

    const caret = document.createElement('span')
    caret.className = 'caret'
    caret.textContent = collapsed ? '▸' : '▾'
    label.append(caret, document.createTextNode(dir))
    label.addEventListener('click', () => {
      if (collapsedFolders.has(folderPath)) collapsedFolders.delete(folderPath)
      else collapsedFolders.add(folderPath)
      renderFileList(currentFiles)
    })

    li.appendChild(label)

    const childList = document.createElement('ul')
    childList.className = 'tree'
    childList.hidden = collapsed

    appendTree(child, folderPath, childList)
    li.appendChild(childList)

    container.appendChild(li)
  }

  for (const path of node.files) {
    const li = document.createElement('li')
    li.className = 'file'
    li.textContent = path.split('/').pop()
    li.dataset.file = path
    li.classList.toggle('active', path === currentFile)
    li.addEventListener('click', () => selectFile(path))

    container.appendChild(li)
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Rebuild the sidebar tree from a flat list of file paths.
const renderFileList = files => {
  currentFiles = files
  fileList.replaceChildren()
  appendTree(buildTree(files), '', fileList)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const loadFileList = async () => {
  const files = await fetch(FILES_URL).then((r) => r.json())
  renderFileList(files)
  return files
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Apply a file list pushed by the server (a .hv file was created or deleted): rebuild the sidebar, and if the file
// being shown has gone, fall back to a default (or clear the diagram when no files remain).
const applyFileList = files => {
  renderFileList(files)

  // If the current selection still exists, keep showing it
  if (files.includes(currentFile)) return

  if (files.length) {
    selectFile(files.includes(DEFAULT_FILE) ? DEFAULT_FILE : files[0])
  } else {
    currentFile = null
    lastSource = null
    app.innerHTML = ''
    transport.innerHTML = ''
    description.innerHTML = ''
    description.removeAttribute('data-shown')
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Entry point
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
await init()

const files = await loadFileList()

if (files.length) {
  await selectFile(files.includes(DEFAULT_FILE) ? DEFAULT_FILE : files[0])
}

// EventSource pushes tagged messages as the hv/ directory changes:
//   { kind: 'file',  file, content } — a file's contents changed; re-render if it's the one we're showing.
//   { kind: 'files', files }         — a file was created or deleted; refresh the sidebar.
// EventSource also reconnects automatically if the connection drops.
const events = new EventSource(EVENTS_URL)

events.onmessage = evt => {
  const msg = JSON.parse(evt.data)

  if (msg.kind === 'file') {
    if (msg.file === currentFile) render(msg.content)
  } else if (msg.kind === 'files') {
    applyFileList(msg.files)
  }
}

events.onerror = () => console.warn('live-reload stream interrupted; reconnecting…')
