// Factorio mod-settings converter — client-side via wasm-bindgen `--target web` glue.
// State model:
//   dataBytes: Uint8Array | null — the current .dat payload (source of truth while in .dat view)
//   textFmt: "json" | "yaml"    — which text format the editor shows
//   textIsFreshDecode: bool     — editor text == current dataBytes' decode
import init from '../pkg/factorio_mod_settings_converter.js';
import {
  dat_to_json, dat_to_yaml, json_to_dat, yaml_to_dat,
} from '../pkg/factorio_mod_settings_converter.js';

const $ = (id) => document.getElementById(id);
const els = {
  drop: $('drop'), fileInput: $('fileInput'), editor: $('editor'),
  formatSeg: $('formatSeg'), formatData: $('formatData'), formatText: $('formatText'),
  fmtNote: $('fmtNote'), status: $('status'),
  btnEncode: $('btnEncode'), btnDecode: $('btnDecode'),
  btnDownloadDat: $('btnDownloadDat'), btnDownloadText: $('btnDownloadText'),
};

await init();

let dataBytes = null;       // current .dat bytes
let dataName = 'mod-settings.dat';
let textFmt = 'json';
let textIsFreshDecode = true;

function setStatus(msg, cls = '') {
  els.status.textContent = msg;
  els.status.className = cls;
}

function ext(name) {
  const n = name.toLowerCase();
  if (n.endsWith('.dat')) return 'dat';
  if (n.endsWith('.json')) return 'json';
  if (n.endsWith('.yaml') || n.endsWith('.yml')) return 'yaml';
  return null;
}

function decodeBytes(bytes) {
  return textFmt === 'json' ? dat_to_json(bytes) : dat_to_yaml(bytes);
}

// ---- view state ------------------------------------------------------
function updateButtons() {
  const hasData = dataBytes !== null;
  els.btnEncode.disabled = !(hasData && els.editor.value !== '');
  els.btnDecode.disabled = !hasData && !(els.editor.value !== '' && dataBytes === null);
  els.btnDecode.disabled = false;
  els.btnDownloadDat.disabled = false;
  els.btnDownloadText.disabled = !(hasData || els.editor.value !== '');
  els.formatSeg.hidden = hasData;
}

function showDecode() {
  try {
    els.editor.value = decodeBytes(dataBytes);
    setStatus(`decoded ${textFmt.toUpperCase()} from ${fmtSize(dataBytes.length)}`, 'ok');
    updateButtons();
  } catch (e) {
    setStatus(`decode failed: ${e}`, 'err');
  }
}

const fmtSize = (n) => n < 1024 ? `${n} B` : `${(n / 1024).toFixed(1)} KB`;

// ---- file loading ----------------------------------------------------
async function loadFile(file) {
  try {
    const fmt = ext(file.name);
    if (!fmt) throw new Error(`unsupported extension: ${file.name}`);
    const buf = new Uint8Array(await file.arrayBuffer());

    if (fmt === 'dat') {
      dataBytes = buf;
      dataName = file.name;
      textIsFreshDecode = true;
      els.fmtNote.textContent = `editing ${textFmt.toUpperCase()} view of ${file.name} (${fmtSize(buf.length)})`;
      els.formatText.classList.toggle('on', false);
      els.formatData.classList.toggle('on', true);
      els.editor.value = '';
      showDecode();
    } else {
      // text input: encode to .dat immediately to validate and store bytes
      const dat = fmt === 'json' ? json_to_dat(new TextDecoder().decode(buf)) : yaml_to_dat(new TextDecoder().decode(buf));
      dataBytes = dat;
      dataName = file.name.replace(/\.(json|ya?ml)$/i, '.dat');
      // show text view of the user's uploaded format
      textFmt = fmt === 'json' ? 'json' : 'yaml';
      els.editor.value = new TextDecoder().decode(buf);
      textIsFreshDecode = false;
      els.fmtNote.textContent = `loaded ${file.name} (${fmtSize(buf.length)}) \u2192 ${dataName} (${fmtSize(dat.length)})`;
      els.formatSeg.hidden = true;
      els.formatData.classList.toggle('on', true);
      els.formatText.classList.toggle('on', false);
      setStatus(`encoded to .dat: ${fmtSize(dat.length)} \u2014 tweak the ${fmt.toUpperCase()} below or download the .dat`, 'ok');
      updateButtons();
    }
  } catch (e) {
    setStatus(`load failed: ${e}`, 'err');
  }
}

// ---- dropzone --------------------------------------------------------
els.drop.addEventListener('click', () => els.fileInput.click());
els.fileInput.addEventListener('change', (ev) => {
  if (ev.target.files[0]) loadFile(ev.target.files[0]);
});
for (const type of ['dragenter', 'dragover']) {
  els.drop.addEventListener(type, (ev) => {
    ev.preventDefault();
    els.drop.classList.add('active');
  });
}
for (const type of ['dragleave', 'drop']) {
  els.drop.addEventListener(type, (ev) => {
    ev.preventDefault();
    els.drop.classList.remove('active');
  });
}
els.drop.addEventListener('drop', (ev) => {
  const f = ev.dataTransfer?.files?.[0];
  if (f) loadFile(f);
});

// ---- format toggle ---------------------------------------------------
els.formatData.addEventListener('click', () => { fmtMode('data'); });
els.formatText.addEventListener('click', () => { fmtMode('text'); });

function fmtMode(mode) {
  if (dataBytes === null) return;
  if (mode === 'data') {
    els.formatData.classList.add('on');
    els.formatText.classList.remove('on');
    els.editor.value = '';
    els.fmtNote.textContent = `raw .dat view: ${dataName} (${fmtSize(dataBytes.length)}) \u2014 switch to text to edit`;
    setStatus('raw .dat bytes are in memory; switch to text to decode+edit', 'ok');
  } else {
    els.formatText.classList.add('on');
    els.formatData.classList.remove('on');
    showDecode();
  }
}

// ---- encode / decode buttons -----------------------------------------
els.btnDecode.addEventListener('click', () => {
  if (dataBytes === null) {
    // allow encoding of the currently edited text back into .dat
    try {
      textFmt = guessFmtFromText(els.editor.value);
      const dat = textFmt === 'json' ? json_to_dat(els.editor.value) : yaml_to_dat(els.editor.value);
      dataBytes = dat;
      dataName = `converted-${Date.now()}.dat`;
      textIsFreshDecode = false;
      els.fmtNote.textContent = `encoded ${fmtSize(dat.length)} from editor text`;
      setStatus(`encoded to ${dataName} (${fmtSize(dat.length)})`, 'ok');
    } catch (e) {
      setStatus(`encode failed: ${e}`, 'err');
    }
  } else if (els.editor.value === '') {
    showDecode();
  } else {
    // re-decode overwriting an edited/encoded .dat
    decodeBytes(dataBytes); // validate only
    showDecode();
  }
  updateButtons();
});

els.btnEncode.addEventListener('click', () => {
  try {
    textFmt = guessFmtFromText(els.editor.value);
    const dat = textFmt === 'json' ? json_to_dat(els.editor.value) : yaml_to_dat(els.editor.value);
    dataBytes = dat;
    els.fmtNote.textContent = `encoded ${fmtSize(dat.length)} to .dat from edited ${textFmt.toUpperCase()}`;
    setStatus(
      textIsFreshDecode && sameBytesQuiet(dat, dataBytes)
        ? `encoded: ${fmtSize(dat.length)} (matches previous bytes)`
        : `encoded: ${fmtSize(dat.length)}`,
      'ok',
    );
    textIsFreshDecode = false;
  } catch (e) {
    setStatus(`encode failed: ${e}`, 'err');
  }
  updateButtons();
});

function guessFmtFromText(text) {
  const t = text.trim();
  return t.startsWith('{') ? 'json' : 'yaml';
}

function sameBytesQuiet(a, b) {
  if (!a || !b || a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

// ---- downloads -------------------------------------------------------
function download(url, name) {
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  URL.revokeObjectURL(url);
}

els.btnDownloadDat.addEventListener('click', () => {
  if (dataBytes) {
    const blob = new Blob([dataBytes], { type: 'application/octet-stream' });
    download(URL.createObjectURL(blob), dataName.endsWith('.dat') ? dataName : dataName + '.dat');
    setStatus('downloaded .dat', 'ok');
  } else {
    setStatus('nothing encoded yet \u2014 use Decode \u2192 / Encode \u2192 .dat first', 'err');
  }
});

els.btnDownloadText.addEventListener('click', () => {
  try {
    const text = els.editor.value !== '' ? els.editor.value : decodeBytes(dataBytes);
    const blob = new Blob([text], { type: 'text/plain' });
    download(URL.createObjectURL(blob), dataName.replace(/\.dat$/i, '') + '.json');
    setStatus('downloaded text format', 'ok');
  } catch (e) {
    setStatus(`download failed: ${e}`, 'err');
  }
});

updateButtons();
