# Vendor import libraries

The Rust native crates load vendor driver DLLs at runtime with Windows `LoadLibrary`/`GetProcAddress`; they do **not** link these import libraries into the product. The `.lib` files are retained here as vendor SDK compatibility artifacts for integrators and for ABI/reference review.

Runtime DLLs remain in the parent `resources/lib/` directory and are the files loaded by the Rust crates:

- PEAK: `PCANBasic.dll`, `PCAN-ISO-TP.dll`, `PLinApi.dll`
- Kvaser: `canlib32.dll`, `linlib.dll`
- ZLG: `zlgcan.dll`
- Vector: `vxlapi64.dll`
- Toomoss: `USB2XXX.dll`
- vSomeIP: `vsomeip3.dll`, `vsomeip3-sd.dll`, `vsomeip3-cfg.dll`, `vsomeip3-e2e.dll`

No C++/SWIG source, `binding.gyp`, or legacy `.node` addon is built from this directory.
