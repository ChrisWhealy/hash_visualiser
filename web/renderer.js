import init, { imports, run_with_imports, drop_scene } from '/pkg/hash_visualiser.js'

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
let currentDeps = new Set() // import paths the current render depends on, so a change to one re-renders this file
let currentFiles = [] // most recent file list, so a folder toggle can re-render the tree
const collapsedFolders = new Set() // folder paths the user has collapsed

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The hv/-root-relative directory of a file path ("sha3/theta.hv" -> "sha3"; "sha256.hv" -> "").
const dirOf = file => (file.includes('/') ? file.slice(0, file.lastIndexOf('/')) : '')

// Resolve an import path written inside a file in `dir`, relative to that directory ("." / ".." segments allowed),
// yielding an hv/-root-relative path. e.g. dir "sha3", rel "theta_c.hv" -> "sha3/theta_c.hv".
const resolvePath = (dir, rel) => {
  const segs = dir ? dir.split('/') : []
  for (const part of rel.split('/')) {
    if (part === '' || part === '.') continue
    else if (part === '..') segs.pop()
    else segs.push(part)
  }
  return segs.join('/')
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Walk the `import` graph of `entrySource` (a file at `entryFile`), fetching the transitive closure of imported files.
// Import paths are resolved relative to the importing file's directory. Returns the parallel path/source arrays that
// run_with_imports expects (keyed by the literal import string the Rust build looks up), plus `deps`, the set of
// hv/-root-relative paths actually fetched (so a live-reload of any of them re-renders this file).
const resolveImports = async (entryFile, entrySource) => {
  const sources = new Map() // literal import string -> source text
  const deps = new Set() // resolved hv/-root-relative paths
  const queue = imports(entrySource).map(literal => ({ literal, dir: dirOf(entryFile) }))

  while (queue.length) {
    const { literal, dir } = queue.shift()
    if (sources.has(literal)) continue

    const resolved = resolvePath(dir, literal)
    const url = fileUrl(resolved)

    let res
    try {
      res = await fetch(url, { cache: 'no-store' })
    } catch (err) {
      throw new Error(`import "${literal}": could not fetch ${url} (${err.message})`)
    }
    if (!res.ok) throw new Error(`import "${literal}": ${url} returned HTTP ${res.status}`)

    const src = await res.text()
    sources.set(literal, src)
    deps.add(resolved)

    // This file's own imports are resolved relative to ITS directory.
    for (const child of imports(src)) {
      if (!sources.has(child)) queue.push({ literal: child, dir: dirOf(resolved) })
    }
  }
  return { paths: [...sources.keys()], sources: [...sources.values()], deps }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const render = async source => {
  // Bail out early if unchanged since last render
  if (source === lastSource) return

  lastSource = source

  // Clear the previous render before drawing afresh (run() appends new <svg> elements).
  app.innerHTML = ''
  transport.innerHTML = ''

  // A stale description may no longer match the edited source: hide it (and forget which node it showed).
  description.innerHTML = ''
  description.removeAttribute('data-shown')

  closeAllModals() // a fresh render of the main diagram invalidates any open expansion overlays

  try {
    const { paths, sources, deps } = await resolveImports(currentFile || '', source)
    currentDeps = deps // resolved paths, so a change to an imported file re-renders this one
    run_with_imports('app', 'transport', source, paths, sources)
    app.dataset.baseFile = currentFile || '' // base dir for resolving this layer's expandable boxes
    banner.style.display = 'none'
    banner.textContent = ''
  } catch (err) {
    banner.style.display = 'block'
    banner.textContent = `Error: ${err}`
    console.error(err)
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Expansion overlays ("modals"). Clicking a box that applies an imported function (marked by the renderer with a
// `data-import` attribute) opens that file's own visualisation in a modal layered over the diagram. The layer beneath
// is blocked by the backdrop, and modals stack — a click inside a modal opens another on top. Each modal renders into
// its own element (so the keyed scene store keeps them all alive) and is released via drop_scene when closed.
const modalStack = []
let modalSeq = 0

const closeTopModal = () => {
  const top = modalStack.pop()
  if (!top) return
  drop_scene(top.appId)
  top.backdrop.remove()
}

const closeAllModals = () => {
  while (modalStack.length) closeTopModal()
}

// Open the file that `importPath` (written inside the file `baseFile`) refers to, in a new modal.
const openModal = async (importPath, baseFile) => {
  const resolved = resolvePath(dirOf(baseFile), importPath)
  const url = fileUrl(resolved)

  let res
  try {
    res = await fetch(url, { cache: 'no-store' })
  } catch (err) {
    throw new Error(`expand "${importPath}": could not fetch ${url} (${err.message})`)
  }
  if (!res.ok) throw new Error(`expand "${importPath}": ${url} returned HTTP ${res.status}`)
  const source = await res.text()
  const { paths, sources } = await resolveImports(resolved, source)

  const n = ++modalSeq
  const appId = `modal-app-${n}`
  const transportId = `modal-transport-${n}`

  const backdrop = document.createElement('div')
  backdrop.className = 'modal-backdrop'
  backdrop.innerHTML =
    `<div class="modal-dialog" role="dialog" aria-label="${resolved}">` +
    `<header class="modal-header"><span class="modal-title">${resolved}</span>` +
    `<button class="modal-close" title="Close (Esc)" aria-label="Close">✕</button></header>` +
    `<div class="modal-body">` +
    `<div id="${appId}" class="canvas" data-base-file="${resolved}"></div>` +
    `<footer id="${transportId}" class="modal-transport"></footer>` +
    `</div></div>`
  document.body.appendChild(backdrop)

  // Close on the ✕ button or on a click in the dimmed area outside the dialog.
  backdrop.querySelector('.modal-close').addEventListener('click', closeTopModal)
  backdrop.addEventListener('click', e => { if (e.target === backdrop) closeTopModal() })

  modalStack.push({ appId, backdrop })

  try {
    run_with_imports(appId, transportId, source, paths, sources)
  } catch (err) {
    closeTopModal()
    throw err
  }
}

// One delegated listener: a click on (or inside) any `[data-import]` box expands it. The box's layer carries the base
// file path (the main canvas, or a modal's canvas) for resolving the import.
document.addEventListener('click', e => {
  const box = e.target.closest('[data-import]')
  if (!box) return
  const layer = e.target.closest('[data-base-file]')
  const baseFile = layer ? layer.dataset.baseFile : currentFile || ''
  openModal(box.dataset.import, baseFile).catch(err => {
    banner.style.display = 'block'
    banner.textContent = `Error: ${err.message || err}`
    console.error(err)
  })
})

document.addEventListener('keydown', e => {
  if (e.key === 'Escape') closeTopModal()
})

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
    await render(source)
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
    // The open file changed — re-render with its pushed content. If one of its *imports* changed, re-select the open
    // file so its dependency closure is re-fetched and merged afresh.
    if (msg.file === currentFile) render(msg.content)
    else if (currentDeps.has(msg.file)) selectFile(currentFile)
  } else if (msg.kind === 'files') {
    applyFileList(msg.files)
  }
}

events.onerror = () => console.warn('live-reload stream interrupted; reconnecting…')
