# Canvas Dock — Design Tokens

## Color

| Token | Value | Use |
| --- | --- | --- |
| `--canvas` | `#0b0b0d` | Application background |
| `--surface` | `#151417` | Rail, dock, and timeline |
| `--surface-deep` | `#111013` | Editor workspace |
| `--field` | `#0e0d10` | Inputs and compact control wells |
| `--border` | `#2b2a2e` | Primary dividers |
| `--border-control` | `#38383d` | Inputs and buttons |
| `--text` | `#f4f0e8` | Primary text |
| `--muted` | `#8f8982` | Supporting text |
| `--quiet` | `#716b65` | Tertiary metadata |
| `--accent` | `#ff6a47` | Primary action and trim |
| `--accent-soft` | `#ff967c` | Accent labels |
| `--selected` | `#72d6a0` | Selected overlay and anchor |
| `--warning` | `#d4a647` | Warning state |

## Typography

- Interface: existing application sans-serif stack headed by Inter.
- Numeric values and time: `ui-monospace, monospace`.
- Project title: `20px`, weight `720`.
- Dock title: `18–19px`, weight `700+`, single-line ellipsis.
- Section label: `9–10px`, uppercase, `0.08–0.12em` tracking.
- Controls and metadata: `10–12px`.

## Geometry

- Editor outer gutter: `12–28px`, responsive to viewport width.
- Rail width: `88px` desktop, `72px` compact laptop.
- Inspector width: `300–320px`.
- Main grid divider: `1px`.
- Rail item minimum height: `58px`.
- Anchor cell: `36–40px`.
- Compact control padding: `8–12px`.
- Border radius: `0`; preserve the existing square visual system.
- Shadows: none.

## Motion

- No new interface motion.
- Respect the existing `prefers-reduced-motion` rule.
- Video and sting preview behavior is unchanged.

## States

- Selected rail item: green border and low-opacity green fill.
- Primary rail action: coral border.
- Disabled action: existing reduced opacity and `not-allowed` cursor.
- Focus: existing 3px coral outline with 3px offset.
- Error, warning, rendering, and success states retain their existing semantic
  colors and messages.
