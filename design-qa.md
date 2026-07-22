# Design QA

- Source visual truth: `/var/folders/z7/x6zjwz7x50g_qxnrvbz0v0pm0000gp/T/codex-clipboard-0ea03069-895e-474e-b5a3-4af3e5f53e92.png`
- Implementation screenshot: unavailable; the debug NSPanel was hidden during desktop capture
- Viewport: source crop 722 × 120 px; desktop implementation
- State: file-transfer conversation header, no connected devices

## Full-view comparison evidence

The source shows an oversized blue feature icon on the left, a heavy two-line title block, and a permanently visible QR code on the right. The implementation removes both large visual anchors, reduces the header to 50 CSS pixels, and moves the QR code into an icon-triggered popover.

## Focused region comparison evidence

The source header crop was inspected at original resolution. A matching post-change crop could not be captured because the Tauri debug NSPanel was not visible to the available capture surface.

## Findings

- [P2] Post-change visual evidence missing
  - Location: file-transfer header
  - Impact: typography and final optical alignment cannot be confirmed from a matched screenshot.
  - Fix: capture the updated header in the same state and compare its 50 px bar, title baseline, status baseline, and QR action alignment.

## Comparison history

- Initial finding: header hierarchy was dominated by a 38 px blue icon and persistent QR asset.
- Fix: removed the leading icon, reduced typography and bar height, replaced persistent QR with a 34 px action and popover.
- Post-fix evidence: production build passed; visual capture remains unavailable.

## Required fidelity surfaces

- Typography: reduced to 12.5 px/600 title and 9.5 px secondary status; pending matched capture.
- Spacing: header reduced to 50 px with 14 px left inset; pending matched capture.
- Colors: neutral toolbar and muted status; accent reserved for interactive QR state.
- Image quality: QR remains a real canvas-rendered QR asset and is shown only in its popover.
- Copy: changed to “局域网文件传输” with address and connection state as secondary text.

## Final result

final result: blocked

Blocker: a matching post-change screenshot of the running Tauri NSPanel is unavailable.
