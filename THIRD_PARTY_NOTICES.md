# Third-party notices

Archipel IDE re-implements some behavior studied in the projects below. No code, assets,
names or strings are copied; the notices travel with the modules that model their behavior.

## Orca (https://github.com/stablyai/orca)

Used for: worktree lifecycle and safety rules (name sanitizing, removal safety, dirty
preflight, orphan proof, include and shared-directory rules, lineage config). Modeled in
`crates/archipel-ide-git/src/worktrees.rs`. Later work packages append further uses here
(agent state detection rules and event mappings, prompt delivery heuristics, notification
cooldown, PTY-host checkpoint design).

```
MIT License

Copyright (c) 2026 Lovecast Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
