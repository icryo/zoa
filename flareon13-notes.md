# Flare-On 13 — solves & lessons learned (player: cryo)

## 1 - FLAreCAPTCHA — `triskaidekaphobia@flare-on.com`
Fake reCAPTCHA HTML. Hold right mouse button 2 s -> flag = XOR of a byte array with key bytes from
(2 * (resetWidget.toString().length + 1115)) & 0xFFFF, plus a Proxy that XORs every element with
document.characterSet.length (5).
- Compute, don't click: lift the JS into node and brute the tiny input space (hold seconds).
- Function.prototype.toString() as key material -> never reformat the source; use original bytes.
- Proxy traps silently transform reads; inspect the handler, not the array in DevTools.
- Validate candidates by the flag suffix @flare-on.com.

## 2 - GhostStream — `y0u_kn0w_n07h1n6_j0n_5n0w@flare-on.com`
WIM image -> GhostStream.exe + PolyjuicePotion.txt with hidden ADS `:LordVoldemort` = "AlbusDumbledore".
Parent: API/strings encrypted with key = exe_name[len/2]^exe_name[0] (filename-dependent! rename = broken).
Flag DAT_140006668 is never set -> must be flipped in a debugger to read the ADS instead of the main
stream. WndProc (WM_CREATE) XORs the buffer with (1 ^ prev_msg=WM_NCCALCSIZE 0x83) and a 15-byte pattern,
only if process age (minutes) > 15*0x29f+0xf = 10080 (= "7 days") -> "HarryPotter". Sent via
\\.\pipe\FlareChallenge to dropped child Partner_CTF.exe (resource BIN/101).
Child: RC4 KSA(key=pipe data), non-standard PRGA (i += 18), out = ks ^ K2[i%19] ^ (18 + ct), where K2 is
"Bxs]bv`^u)TpcnolVjN" decoded in place by a temporary inline patch of ucrtbase!strlen -> "Piertotum Locomotor".
Decoys: OVERRIDE: TCP listener on 127.0.0.1:31337, SESSION_TOKEN_CACHE env var, Telemetry string,
COMPUTERNAME/IsDebuggerPresent/GetTickCount calls whose results are unused.
- `7z x` extracts NTFS alternate data streams from WIM as `file:stream`; wimlib `dir --detailed` lists them.
- Brute-force the string decryptor (3 modes x 256 keys) instead of reversing the SIMD-mangled mode selector.
- Decompiler + my own reading got one small constant wrong (length 18 vs 19). Once the model is
  "almost right", sweep the uncertain parameters (length, step, hook on/off, prev-message byte) instead of
  re-reading asm for an hour. Brute search over ~1.4k keys x 8 model variants took <1 s.
- wine runs these console/GUI/named-pipe samples fine for quick behavioural checks (no ADS on Linux fs).
- Beware "story" constants: 10080 minutes = 7 days = the potion text; names hint the keys (Harry Potter theme).

## Tooling notes
- ctfd.io file links redirect to S3: use `curl -L`. Register/login have real reCAPTCHA -> human logs in once,
  then a CTFd access token (`Authorization: Token ctfd_...`) drives /api/v1 (list, download, submit).
- GitHub release downloads blocked by the sandbox proxy; Ghidra 12.3 built from source (JDK 25, deps served
  from a local ghidra-data clone via http.server, Z3 extension dropped). `gdec <bin> [regex]` = headless decompile.

## 3 - FlareOn13.doc — `Ju$7_4_l177l3_7r34$ur3_HUN7@flare-on.com`
One file, six formats; each layer yields a fragment. A Crystal arm64 checker (fat Mach-O slice 2) MD5-matches
6 inputs (order-independent) against nibble-encoded digests (^0x80), then RC4-decrypts a 40-byte blob with
key = XOR of SHA-256(fragment_i).
1. Custom printable 16-bit COM prologue (not real EICAR) -> self-patches `int 21h/ah=9` -> prints
   `FLARE-STANDARD-ANTIVIRUS-TEST-FILE!` (unicorn 16-bit emulation with an INT hook).
2. PDF (RC4-40, empty user pw) invisible text (Tr 3) in EBCDIC cp037 -> `mainframe_ebcdic_ghost`.
3. JBIG2 image drawn off-page (-80,-80) -> `jbig2_fax_geometry`.
4. UDF disc stored as raw 2352-byte CD sectors (sector data at +16): convert to 2048/sector before 7z.
   `udf_tagged_descriptor` was a plain string in the image.
5. Trailing ZIP: ZipCrypto pw `infected` (decrypted header 11 22 33 ... = tell), method 2 "Reduced"
   (no common tool supports it -> wrote an unreduce) -> `reduce_not_deflate`.
6. Mach-O slice 1: ASCII fire animation with a SHA-1-like (no-rotate schedule) value XORed to
   `Ød1n_373rn4l_fl4r3` — UTF-8 `\xc3\x98`! I first misread those bytes as garbage.
- Get the list of expected hashes first; then hash every string/token/EBCDIC/UTF-16 run from every layer.
  It turns layer hunting into a checklist.
- When a hand port gives "almost readable" output, emulate the real code (unicorn arm64 on the mapped
  Mach-O, skipping `bl`s to imports) instead of guessing. Non-ASCII bytes may just be UTF-8.
- Crystal string literals = {type_id=1, bytesize, size, bytes}; the author leetified all stdlib strings.
- Weird ZIP methods (Reduce/Shrink/Implode) are a known Flare trick; keep a decoder around.

## 4 - ToxicMiner — `cpu_m3lt3d_4nd_all_i_g0t_w4s_th1s_fl4g@flare-on.com` (volume serial 0x01CEC01D)
Modified cpuminer-multi 1.3.7. Added code found fast by decompiling only callers of unusual imports
(GetVolumeInformationA) instead of analysing 1.8 MB of miner code. Custom scanhash: sha256d(volume_serial ||
nonce) == hexdecode(rpc_user `-u`, 64 hex). On success: key = sha256d(serial || 0x0b501e7e)[:16], RC4-decrypt
51 bytes at 0x1401bd3b0 -> "Accepted: <flag>". Default pass base64 = "m1n3_y0ur_0wn_bus1n3ss".
- The only real unknown is the 32-bit volume serial -> brute 2^32 in C (pthreads) with known-plaintext
  suffix "@flare-on.com". No need to know the -u target or to actually mine.
- Ghidra: global refs from rip-relative code were missed; a raw disp32 scan with capstone found the writers
  (cpuminer option parser: case 'u' -> rpc_user).
- My first brute (textbook SHA-256) found nothing after 28 min: the transform is custom (32 rounds, swapped
  K order, final S-box/XOR tweak on state[0]). Lesson: validate a reimplementation against the real code on
  one input (unicorn) BEFORE burning a 2^32 search.
- Best trick: mmap the PE image at its ImageBase on Linux and call the original function through an
  `__attribute__((ms_abi))` pointer. Native speed, exact semantics, no reimplementation (23 min for 2^32).

## 5 - catthief — `th3r3_b3_tr34sure_1n_th4t_pc4p@flare-on.com`
Rust (ureq) exfil tool + pcap of 6 `POST /exfil` to 192.168.56.102:8080. Every message (both directions) =
RC4(key = 25 bytes XOR 0x33 -> "i11_b3_p1und3r1n_y3r_d474", fresh state per message) over a custom
bit-packed canonical-Huffman container: 11-bit table size (256) | 32-bit payload byte count | 256 x 8-bit
code lengths | code bits. Server replies are the next file path to steal; five cat JPEGs exfiltrated,
flag is written on a scroll in catcatcat.jpg.
- Identical ciphertext prefixes across messages = keystream reuse; confirms a fixed-key stream cipher early.
- tshark `--export-objects http,dir` pulls all bodies in both directions in one go.
- Validate the container on the tiny message first ("READY") before decoding MBs.
- Ghidra again missed rip-relative xrefs to strings; my capstone disp32 scanner (`/opt/re/scripts/ripref.py`)
  finds them in seconds. Should be the default first step.
- Flags can be in images: view outputs, and crop at full resolution to read exact characters.

## 6 - Threat Invaders — SOLVED
.NET single-file self-contained bundle (`ThreatInvaders.exe`, Space-Invaders theme) that drops/loads
native helper DLLs (`embedded.dll` / `real.dll`) and talks over the network (`challenge.pcapng`).
- Unpack the single-file bundle first (extract the embedded managed + native assemblies) before any
  decompilation — the interesting logic is in the app assembly + the native DLL, not the .NET host.
- Pair the static logic with the captured traffic (`.pcapng`): the DLL's crypto/protocol is easiest to
  confirm against real packets rather than reasoning about it in isolation.
- Treat game state / score thresholds as gates the same way as the Ghost Stream "7 days" timer: story
  constants often encode the unlock condition.

## 7 - FlareCalc — `t3mplat3s_asn_and_j1t_0h_my@flare-on.com`
Native calculator (`FlareCalc.exe`) whose checker is built from C++ expression **templates**, an **ASN**
(abstract-syntax) evaluator, and a small **JIT** — as the flag itself spells out. Solved by decompiling
(`FlareCalc_decompiled.c`) and iteratively patching (`FlareCalc_patched*.exe`) to observe/bypass stages,
plus a state dump (`states_all.txt`) to follow the evaluator; runs under wine for quick behavioural checks
(`wine_out.txt`).
- When the flag/theme names the mechanism (templates/ASN/JIT), let that steer where to look instead of
  reading the whole binary top-down.
- Iterative binary patching to force/observe each evaluator stage is faster than fully modelling a JIT.

## 8 - crux — UNSOLVED (deep analysis; challenge 9 stays locked behind it)
Manifest-V3 browser extension whose `check(input)` lives in a **Go 1.26 WASM** binary built with **garble**
(control-flow flattening + string-literal virtualization). Wrong input returns the literal `"Bad input"`.

What was proven this session (correcting several earlier wrong assumptions):
- The JS bridge trace shows a wrong input triggers only `valuePrepareString(input)` → `stringVal("Bad input")`;
  all decrypt/validate work is internal to WASM. On success the same path would emit `stringVal(<flag>)`.
- `check` is WASM `func 2362`, a garble state machine: 224 nested blocks, a 656-entry `br_table`, state =
  `continuation_id & 0xFFFF` (the low 16 bits of the `i64` continuation constants like `0x1926_0xxx`).
- **func 2324 (table[6400]) is NOT the validator.** Trapping it (and every AES-format input) still returns
  "Bad input"; CDP confirms the call_indirect at 0x27c51b and the presumed flag block at 0x27c61c are never
  reached. The old "AES-256-CBC key/IV + 28-byte buffer, func 954 compares 25 bytes" model is wrong:
  func 954 is a character-class table lookup (table @0x50EE0) used at init.
- Real wrong-input control flow (via CDP; wasm `columnNumber` == wasm-objdump file offset):
  `0x27b21c (builder) → loop @0x27d19a calling func 2374 (per-element) → decision → func 2372 @0x27d3c8`.
  `func 2372` is a **generic string builder** (hit ~twice per check and heavily at init) — patching it does
  nothing. The "state 624 = error / 636 = success" labels from earlier sessions are also wrong (624's guard
  is Go's write-barrier flag @0x227010).
- `call 674` is garble's **universal literal decoder** (fixed pool @0x43B62, `sync.Once`-guarded one-time
  init via func 672→1280→1336/703, a coroutine-virtualized decoder). 60 sites in func 2362 + 32 in func 2374
  decode *all* literals, "Bad input" included. The flag literal never appears in memory after init or a
  wrong-input check (searched all linear memory for `@flare-on`), i.e. it is decoded/derived only on success.
- Forcing the flag path by redirecting a state constant (e.g. loop-done → state 400/label 91) **traps**:
  those states are garble local-spill/bounds-grow handlers, not clean flag-build entries; the 60 `call 674`
  flag appends are scattered 0x27c645–0x27d4dc, interleaved with the validation loop.

Why it's hard / where the flag is gated: the actual per-element constraint lives inside `func 2374`, itself a
325-entry `br_table` garble state machine, and the decoder is coroutine-virtualized — so neither a clean
"force success" patch nor a static pool decode works without more work.

Tooling that worked (reusable):
- CDP over headless Chromium (`--load-extension`, or a tiny static server + `popup.html`) gives real wasm
  breakpoints, single-step (`Debugger.stepOver`), and `Debugger.evaluateOnCallFrame` on a JS frame to read
  `window._wasmInstance.exports.mem` **while paused** (the page itself can't run JS while paused).
  Key: CDP wasm `location.columnNumber` equals the wasm-objdump absolute file offset.
- `functionBodyOffsets` from `Debugger.disassembleWasmModule` is streamed/partial — don't rely on it; use
  file offsets from `wasm-objdump -d` directly.
- Trap-probing (patch a function's first byte to `unreachable`) to test reachability — but it can't
  distinguish init-time from check-time calls, so CDP breakpoints (set after init) are the reliable
  discriminator.
- Node harness: Go's `importObject` namespace is `gojs`; stub `document/window/crypto/performance`; `check`
  is exposed on `globalThis` after `go.run`.

Next moves if resumed (in order of promise):
1. Extract the per-element constraint from `func 2374` (differential CDP: vary one input byte, diff the
   executed instruction path to find the input-dependent branch), then constraint-solve the input.
2. Reach the flag-build states cleanly by entering the pre-append setup state (not the spill handlers) with a
   valid `strings.Builder`, then let the coroutine walk the 60 `call 674` appends and capture `stringVal`.
3. Replicate garble's coroutine decoder (func 1280/1336/703) over the pool @0x43B62 to decode literals offline.

## Meta lessons (this session)
- Re-verify inherited "facts" before building on them: multiple confidently-stated findings from earlier
  sessions (the validator function, the AES key/IV model, the error/success state numbers) were wrong, and
  every hour spent patching those was wasted. A single reachability probe (trap the function, see if output
  changes) would have caught it immediately.
- For garble/obfuscated WASM, dynamic CDP tracing (breakpoints + step + live memory) beats static
  disassembly reading — the control flow is flattened and the decoders are virtualized.
- Know when to checkpoint: one crackme should not consume a whole engagement. Map it, capture reusable
  tooling and an honest "here's exactly where the gate is", and report rather than churn.
