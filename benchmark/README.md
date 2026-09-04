# Chirograph benchmark corpus

`benchmark/` contains data-only, hermetic benchmark cases. Normal benchmark execution analyzes only files under each case's `fixture/` directory. Provenance, licensing, and golden truth are benchmark-maintenance data and are not analyzer inputs.

## Third-party fixture licensing

### Cargo

Fixture files under `benchmark/cargo/` are verbatim source files from `rust-lang/cargo` at revisions declared in each `specimen.yaml`. Cargo declares `MIT OR Apache-2.0`. These benchmark copies are redistributed under the MIT option.

MIT permission notice from Cargo's `LICENSE-MIT` at revision `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`:

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
