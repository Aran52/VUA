# Supplemental license provenance

These texts cover dependencies whose published source packages do not include the complete
license text. The per-build collector fails for any other dependency with no license text.

| File | Exact source and treatment |
| --- | --- |
| LICENSE-vrc-get-vpm.txt | Original MIT notice from [vrc-get source revision 3d5a0f9d208ef715a8871e02dbb135f6303eab6f](https://raw.githubusercontent.com/anatawa12/vrc-get/3d5a0f9d208ef715a8871e02dbb135f6303eab6f/LICENSE), matching the 0.0.16 package's `.cargo_vcs_info.json` |
| LICENSE-vrc-get-litedb.txt | The 0.3.0-beta.8 package and [source revision 51d608aa11c2fd90a70bf59a19beafd9b214ec81](https://github.com/anatawa12/litedb-rs/commit/51d608aa11c2fd90a70bf59a19beafd9b214ec81) declare MIT and list the authors but ship no standalone license. Preserve that declaration and unmodified source; reproduce the MIT permission text without inventing an upstream copyright line |
| LICENSE-pdf-extract.txt | The 0.10.0 package and [source revision 8ea76f95eaed2de45c9337830f3fdd5c0a862292](https://github.com/jrmuizel/pdf-extract/commit/8ea76f95eaed2de45c9337830f3fdd5c0a862292) declare MIT and name the author but ship no standalone license, and the repository has none. Same treatment as LiteDB: unmodified source supplied, MIT permission text reproduced without an invented copyright line |
| LICENSE-adobe-cmap-parser.txt | The 0.4.1 package and [source revision ac107d55f0d31a9d47955082238ab9e4fb157cdb](https://github.com/jrmuizel/adobe-cmap-parser/commit/ac107d55f0d31a9d47955082238ab9e4fb157cdb) declare MIT and name the author but ship no standalone license, and the repository has none. Same treatment as LiteDB: unmodified source supplied, MIT permission text reproduced without an invented copyright line |
| LICENSE-type1-encoding-parser.txt | The 0.1.1 package and [source revision 55d193e28c9a5909be02a5b1f91fd3f76b076747](https://github.com/jrmuizel/type1-encoding-parser/commit/55d193e28c9a5909be02a5b1f91fd3f76b076747) declare MIT and name the author but ship no standalone license, and the repository has none. Same treatment as LiteDB: unmodified source supplied, MIT permission text reproduced without an invented copyright line |
| LICENSE-MPL-2.0.txt | Complete [Mozilla Public License 2.0](https://www.mozilla.org/media/MPL/2.0/index.txt); the selectors 0.38.0 source headers declare this license. All resolved MPL source packages are supplied unmodified in the generated notices |

Do not reuse a supplemental mapping for a new dependency version without checking its actual
source and terms. Electron/Chromium/Node and Rust standard-library notices are copied from the
exact installed distributions; the source/inventory collector is not a claim of legal advice
or redistribution of the externally installed VRChat tools.
