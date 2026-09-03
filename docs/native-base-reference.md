# Native migration archive

The runtime contains Rust native addons only. Legacy C/C++/SWIG sources were removed from the working tree as required by the zero-C++ target. Their last committed form remains available through Git history (`git log` / `git show`) and is not copied into the product.

Rust implementations are under `native/`; TypeScript callers are under `src/main/`. The migration contract is the exported API consumed by those callers and is checked by `scripts/check-native-api.mjs`.

Verification:

```bash
node scripts/check-native-api.mjs
node scripts/verify-zero-cpp.mjs
node scripts/build-native.mjs
npm run build:sidecar
```
