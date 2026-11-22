/// Wall-clock time in the user's time zone, as the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

pub fn default_file_name(time: LocalTime) -> String {
    format!(
        "Captura_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}.png",
        time.year, time.month, time.day, time.hour, time.minute, time.second
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_carries_the_date_and_time_with_fixed_width_fields() {
        let time = LocalTime {
            year: 2026,
            month: 3,
            day: 7,
            hour: 9,
            minute: 5,
            second: 1,
        };

        assert_eq!(default_file_name(time), "Captura_2026-03-07_09-05-01.png");
    }

    #[test]
    fn name_keeps_two_digit_fields_untouched() {
        let time = LocalTime {
            year: 2026,
            month: 11,
            day: 22,
            hour: 23,
            minute: 59,
            second: 58,
        };

        assert_eq!(default_file_name(time), "Captura_2026-11-22_23-59-58.png");
    }
}
