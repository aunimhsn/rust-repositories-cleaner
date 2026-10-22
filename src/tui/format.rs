//! Formatting of repository values for display.

use i18n_embed::fluent::FluentLanguageLoader;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::i18n::fl;

const KILOBYTES_PER_MEGABYTE: u64 = 1024;
const KILOBYTES_PER_GIGABYTE: u64 = 1024 * 1024;

/// Human-readable size: whole kilobytes or megabytes, gigabytes with one decimal.
pub fn size(loader: &FluentLanguageLoader, size_kb: u64) -> String {
    if size_kb < KILOBYTES_PER_MEGABYTE {
        return fl!(loader, "size-kilobytes", size = size_kb);
    }
    let megabytes = (size_kb + KILOBYTES_PER_MEGABYTE / 2) / KILOBYTES_PER_MEGABYTE;
    // Sizes rounding up to 1024 MB are shown in gigabytes.
    if megabytes < KILOBYTES_PER_MEGABYTE {
        return fl!(loader, "size-megabytes", size = megabytes);
    }
    // Rounded to the nearest tenth; the decimal separator depends on the language.
    let tenths = (size_kb * 10 + KILOBYTES_PER_GIGABYTE / 2) / KILOBYTES_PER_GIGABYTE;
    let (whole, tenth) = (tenths / 10, tenths % 10);
    fl!(loader, "size-gigabytes", whole = whole, tenth = tenth)
}

/// Date of `timestamp` in `time_zone`, in absolute format (`2026-10-22`).
pub fn date(timestamp: Timestamp, time_zone: &TimeZone) -> String {
    timestamp.to_zoned(time_zone.clone()).date().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n;
    use crate::language::Language;

    #[test]
    fn small_sizes_are_in_kilobytes() {
        let loader = i18n::load(Language::English).unwrap();

        assert_eq!(size(&loader, 0), "0 KB");
        assert_eq!(size(&loader, 1023), "1023 KB");
    }

    #[test]
    fn medium_sizes_are_rounded_megabytes() {
        let loader = i18n::load(Language::English).unwrap();

        assert_eq!(size(&loader, 1024), "1 MB");
        assert_eq!(size(&loader, 1536), "2 MB");
        assert_eq!(size(&loader, 500 * 1024), "500 MB");
    }

    #[test]
    fn sizes_just_under_a_gigabyte_are_shown_in_gigabytes() {
        let loader = i18n::load(Language::English).unwrap();

        assert_eq!(size(&loader, 1023 * 1024), "1023 MB");
        assert_eq!(size(&loader, KILOBYTES_PER_GIGABYTE - 1), "1.0 GB");
    }

    #[test]
    fn large_sizes_are_gigabytes_with_a_localized_decimal() {
        let english = i18n::load(Language::English).unwrap();
        let french = i18n::load(Language::French).unwrap();
        let size_kb = 5 * 1024 * 1024 + 512 * 1024;

        assert_eq!(size(&english, size_kb), "5.5 GB");
        assert_eq!(size(&french, size_kb), "5,5 Go");
        assert_eq!(size(&french, 1024), "1 Mo");
    }

    #[test]
    fn date_is_absolute_in_the_given_time_zone() {
        let timestamp: Timestamp = "2026-10-22T23:30:00Z".parse().unwrap();

        assert_eq!(date(timestamp, &TimeZone::UTC), "2026-10-22");
        assert_eq!(
            date(timestamp, &TimeZone::fixed(jiff::tz::offset(2))),
            "2026-10-23"
        );
    }
}
