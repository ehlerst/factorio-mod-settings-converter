// Node round-trip verification: dat -> json -> dat must be byte-identical,
// same guarantee as `make integration` proves natively.
import { initSync, dat_to_json, dat_to_yaml, yaml_to_dat, json_to_dat } from '../pkg/factorio_mod_settings_converter.js';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { Buffer } from 'node:buffer';

// Node 21 can't fetch(file://), so init synchronously from the wasm bytes.
// Paths are resolved relative to this script via import.meta.url, not CWD.
initSync({ module: readFileSync(new URL('../pkg/factorio_mod_settings_converter_bg.wasm', import.meta.url)) });

const bytes = readFileSync(new URL('../mod-settings.dat', import.meta.url));
const md5 = (b) => createHash('md5').update(b).digest('hex');
console.log('input size', bytes.length, 'md5', md5(bytes));

const json = dat_to_json(new Uint8Array(bytes));
console.log('json sample:', json.slice(0, 120).replace(/\n/g, ''), '...');
const roundJson = json_to_dat(json);
console.log('json round-trip identical:', Buffer.compare(Buffer.from(roundJson), bytes) === 0,
  'md5', md5(Buffer.from(roundJson)));

const yaml = dat_to_yaml(new Uint8Array(bytes));
console.log('yaml sample:', JSON.stringify(yaml.slice(0, 80)));
const roundYaml = yaml_to_dat(yaml);
console.log('yaml round-trip identical:', Buffer.compare(Buffer.from(roundYaml), bytes) === 0,
  'md5', md5(Buffer.from(roundYaml)));

// error path check
try { dat_to_json(new Uint8Array([1, 2, 3])); } catch (e) {
  console.log('error path works:', String(e).slice(0, 80));
}
writeFileSync('/tmp/converter_check.json', json);
