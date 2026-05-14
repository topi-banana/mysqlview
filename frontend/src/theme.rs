//! Tailwind class helpers derived from DESIGN.md.
//!
//! Centralising these strings keeps the design system consistent and makes it
//! trivial to retheme later (e.g. dark mode).

pub const BTN_PRIMARY: &str = "inline-flex items-center justify-center px-4 py-2 rounded-[6px] bg-primary text-white \
     font-medium text-sm hover:bg-primary-hover transition-all duration-150 \
     hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(99,102,241,0.35)] \
     disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:translate-y-0 \
     disabled:hover:shadow-none";

pub const BTN_SECONDARY: &str = "inline-flex items-center justify-center px-4 py-2 rounded-[6px] border border-border \
     bg-surface text-text font-medium text-sm hover:border-text-secondary \
     transition-all duration-150 hover:-translate-y-px";

pub const BTN_GHOST: &str = "inline-flex items-center justify-center px-3 py-1.5 rounded-[6px] text-text-secondary \
     font-medium text-sm hover:text-text transition-colors";

pub const BTN_DESTRUCTIVE: &str = "inline-flex items-center justify-center px-4 py-2 rounded-[6px] border border-error/40 \
     bg-surface text-error font-medium text-sm hover:bg-error/5 transition-all duration-150 \
     hover:-translate-y-px disabled:opacity-50 disabled:cursor-not-allowed \
     disabled:hover:translate-y-0";

pub const CARD: &str = "block bg-surface border border-border rounded-[12px] overflow-hidden \
     transition-all duration-200 hover:-translate-y-0.5 \
     hover:shadow-[0_8px_30px_rgba(0,0,0,0.08)]";

pub const CARD_FLAT: &str = "bg-surface border border-border rounded-[12px] overflow-hidden";

pub const INPUT: &str = "w-full px-3.5 py-2.5 text-sm rounded-[6px] border border-border bg-surface \
     placeholder:text-neutral focus:outline-none focus:border-primary \
     focus:ring-[3px] focus:ring-primary/10 transition-colors";

pub const CHIP_NEUTRAL: &str = "inline-flex items-center px-3 py-1 rounded-full bg-neutral/10 text-text-secondary \
     text-xs font-medium";

pub const CHIP_PRIMARY: &str =
    "inline-flex items-center px-3 py-1 rounded-full bg-primary text-white text-xs font-medium";

pub const CHIP_SUCCESS: &str = "inline-flex items-center px-3 py-1 rounded-full bg-success/10 text-success text-xs font-medium";

pub const CHIP_WARNING: &str = "inline-flex items-center px-3 py-1 rounded-full bg-warning/10 text-warning text-xs font-medium";

pub const CHIP_ERROR: &str =
    "inline-flex items-center px-3 py-1 rounded-full bg-error/10 text-error text-xs font-medium";

pub const SECTION_HEADING: &str = "text-3xl font-display font-semibold tracking-tight";
#[allow(dead_code)]
pub const SUBHEAD: &str = "text-xl font-display font-semibold tracking-tight";
pub const OVERLINE: &str =
    "text-[11px] uppercase tracking-[0.08em] font-medium text-text-secondary";

pub fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}
