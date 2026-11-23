use sushot_core::{Action, Mode, Output};

pub(crate) const USAGE: &str = "usage: sushot [--region | --full | --window] [--save]";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Start the resident application.
    Run,
    /// Ask the resident application to take a capture.
    Capture(Action),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UsageError(pub(crate) String);

pub(crate) fn parse<I, S>(arguments: I) -> Result<Command, UsageError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut mode = None;
    let mut save = false;
    for argument in arguments {
        let requested = match argument.as_ref() {
            "--region" => Some(Mode::Region),
            "--full" => Some(Mode::FullScreen),
            "--window" => Some(Mode::Window),
            "--save" => {
                save = true;
                None
            }
            other => return Err(UsageError(format!("unknown argument `{other}`"))),
        };
        if let Some(requested) = requested {
            if mode.is_some() {
                return Err(UsageError(
                    "only one of --region, --full and --window".into(),
                ));
            }
            mode = Some(requested);
        }
    }
    match (mode, save) {
        (None, false) => Ok(Command::Run),
        (None, true) => Err(UsageError(
            "--save needs --region, --full or --window".into(),
        )),
        (Some(mode), save) => Ok(Command::Capture(Action {
            mode,
            output: if save {
                Output::SaveDialog
            } else {
                Output::Clipboard
            },
        })),
    }
}

/// One line sent to the resident application, for example `full save`.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn encode_request(action: Action) -> String {
    let mode = match action.mode {
        Mode::Region => "region",
        Mode::FullScreen => "full",
        Mode::Window => "window",
    };
    match action.output {
        Output::Clipboard => mode.to_owned(),
        Output::SaveDialog => format!("{mode} save"),
    }
}

#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn decode_request(line: &str) -> Option<Action> {
    let mut words = line.split_whitespace();
    let mode = match words.next()? {
        "region" => Mode::Region,
        "full" => Mode::FullScreen,
        "window" => Mode::Window,
        _ => return None,
    };
    let output = match words.next() {
        None => Output::Clipboard,
        Some("save") => Output::SaveDialog,
        Some(_) => return None,
    };
    words.next().is_none().then_some(Action { mode, output })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(mode: Mode, output: Output) -> Command {
        Command::Capture(Action { mode, output })
    }

    #[test]
    fn no_arguments_start_the_application() {
        assert_eq!(parse(Vec::<&str>::new()), Ok(Command::Run));
    }

    #[test]
    fn each_mode_flag_selects_a_clipboard_capture() {
        assert_eq!(
            parse(["--region"]),
            Ok(action(Mode::Region, Output::Clipboard))
        );
        assert_eq!(
            parse(["--full"]),
            Ok(action(Mode::FullScreen, Output::Clipboard))
        );
        assert_eq!(
            parse(["--window"]),
            Ok(action(Mode::Window, Output::Clipboard))
        );
    }

    #[test]
    fn save_switches_the_output_in_any_position() {
        assert_eq!(
            parse(["--save", "--full"]),
            Ok(action(Mode::FullScreen, Output::SaveDialog))
        );
        assert_eq!(
            parse(["--window", "--save"]),
            Ok(action(Mode::Window, Output::SaveDialog))
        );
    }

    #[test]
    fn invalid_combinations_are_rejected() {
        assert!(parse(["--save"]).is_err());
        assert!(parse(["--region", "--full"]).is_err());
        assert!(parse(["--region", "--region"]).is_err());
        assert!(parse(["--bogus"]).is_err());
        assert!(parse(["region"]).is_err());
    }

    #[test]
    fn requests_round_trip_for_all_six_actions() {
        for mode in [Mode::Region, Mode::FullScreen, Mode::Window] {
            for output in [Output::Clipboard, Output::SaveDialog] {
                let action = Action { mode, output };

                assert_eq!(decode_request(&encode_request(action)), Some(action));
            }
        }
    }

    #[test]
    fn malformed_requests_are_ignored() {
        assert_eq!(decode_request(""), None);
        assert_eq!(decode_request("pizza"), None);
        assert_eq!(decode_request("full burn"), None);
        assert_eq!(decode_request("full save now"), None);
    }
}
