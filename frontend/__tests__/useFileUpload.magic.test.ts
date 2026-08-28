import { detectContentError, getExt, readHeadBytes, WASM_MAGIC } from '../hooks/useFileUpload';

function makeFile(parts: BlobPart[], name: string, type = ''): File {
  return new File(parts, name, { type });
}

describe('getExt', () => {
  it('returns the lower-cased extension including the dot', () => {
    expect(getExt('Contract.WASM')).toBe('.wasm');
    expect(getExt('src/lib.rs')).toBe('.rs');
  });
});

describe('readHeadBytes', () => {
  it('reads only the requested number of leading bytes', async () => {
    const file = makeFile([new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8])], 'blob.bin');
    const head = await readHeadBytes(file, 4);
    expect(Array.from(head)).toEqual([1, 2, 3, 4]);
  });
});

describe('detectContentError – client-side magic byte validation', () => {
  it('accepts a valid .wasm file (starts with \\0asm)', async () => {
    const bytes = new Uint8Array([...WASM_MAGIC, 0x01, 0x00, 0x00, 0x00]);
    const file = makeFile([bytes], 'contract.wasm', 'application/wasm');
    expect(await detectContentError(file, '.wasm')).toBeNull();
  });

  it('rejects a .wasm that is really an .exe renamed (MZ header)', async () => {
    // 0x4D 0x5A = "MZ", the DOS/PE executable signature.
    const bytes = new Uint8Array([0x4d, 0x5a, 0x90, 0x00, 0x03, 0x00]);
    const file = makeFile([bytes], 'contract.wasm', 'application/wasm');
    expect(await detectContentError(file, '.wasm')).toBe(
      'This file does not appear to be a valid WASM binary'
    );
  });

  it('rejects an empty file', async () => {
    const file = makeFile([], 'contract.wasm');
    expect(await detectContentError(file, '.wasm')).toBe('File is empty');
  });

  it('rejects a truncated .wasm file with an incomplete magic header', async () => {
    const file = makeFile([new Uint8Array([0x00, 0x61])], 'contract.wasm');
    expect(await detectContentError(file, '.wasm')).toBe(
      'This file does not appear to be a valid WASM binary'
    );
  });

  it('accepts a valid .rs source file', async () => {
    const file = makeFile(['pub fn main() {\n    // entry point\n}\n'], 'lib.rs', 'text/plain');
    expect(await detectContentError(file, '.rs')).toBeNull();
  });

  it('accepts a .rs file that opens with a doc comment', async () => {
    const file = makeFile(['//! crate docs\nuse std::io;\n'], 'lib.rs', 'text/plain');
    expect(await detectContentError(file, '.rs')).toBeNull();
  });

  it('rejects a .rs file whose content is a binary blob', async () => {
    const file = makeFile([new Uint8Array([0x00, 0x01, 0x02, 0x03, 0xff, 0xfe])], 'lib.rs');
    expect(await detectContentError(file, '.rs')).toBe(
      'This file does not appear to be valid Rust source'
    );
  });

  it('does not magic-check types without a reliable signature (.toml)', async () => {
    const file = makeFile(['[package]\nname = "x"\n'], 'Cargo.toml', 'text/plain');
    expect(await detectContentError(file, '.toml')).toBeNull();
  });
});
