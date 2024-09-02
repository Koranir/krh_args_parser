use std::borrow::Cow;

pub trait ArgumentParser: Default {
    /// Get the application version.
    fn version(&self) -> Cow<'static, str> {
        concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION")).into()
    }

    /// Get your help message.
    /// It is recommended to also print your version here using [`version`].
    fn help(&self) -> Cow<'static, str>;

    /// Handle a long flag.
    fn long(
        &mut self,
        long: &str,
        next: &mut dyn FnMut() -> Result<String, Cow<'static, str>>,
    ) -> Result<(), Cow<'static, str>>;
    /// Handle a short flag.
    fn short(
        &mut self,
        short: char,
        is_last: bool,
        next: &mut dyn FnMut() -> Result<String, Cow<'static, str>>,
    ) -> Result<(), Cow<'static, str>>;

    /// Handle a positional argument.
    /// return true to retry as a subcommand, giving the rest of the arguments as that of the subcommand.
    fn argument(
        &mut self,
        arg: &str,
        next: &mut dyn FnMut() -> Result<String, Cow<'static, str>>,
    ) -> Result<bool, Cow<'static, str>>;

    /// Handle a subcommand with the given arguments.
    fn subcommand(
        &mut self,
        command: &str,
        command_args: Box<dyn Iterator<Item = String>>,
    ) -> Result<(), Cow<'static, str>>;
}

pub fn parse_args<P: ArgumentParser>() -> Result<P, Cow<'static, str>> {
    // Skip the first argument, which is always the program name.
    parse(std::env::args().skip(1))
}

pub fn parse<P: ArgumentParser>(
    args: impl Iterator<Item = String> + 'static,
) -> Result<P, Cow<'static, str>> {
    let mut parser = P::default();

    let mut args = args.peekable();

    if args.peek().is_none() {
        return Err("No arguments given".into());
    }

    while let Some(arg) = args.next() {
        if let Some(s) = arg.strip_prefix("--") {
            match s.split_once('=') {
                Some((long, next)) => {
                    let mut taken = false;
                    let mut nextfn = || {
                        taken = true;
                        Ok(next.into())
                    };
                    parser.long(long, &mut nextfn)?;

                    if !taken {
                        return Err(format!(
                            "Flag '{long}' was given argument '{next}' without using it"
                        )
                        .into());
                    }
                }
                None => {
                    let mut nextfn = || match args.peek() {
                        Some(f) => {
                            if f.starts_with('-') {
                                Err(format!("Expected value, got flag {f}").into())
                            } else {
                                Ok(args.next().unwrap())
                            }
                        }
                        None => Err("Expected value but no arguments were left".into()),
                    };

                    parser.long(s, &mut nextfn)?;
                }
            }
        } else if let Some(s) = arg.strip_prefix('-') {
            let mut peekable = s.chars().peekable();
            while let Some(c) = peekable.next() {
                match peekable.peek() {
                    Some(&p) => {
                        if p == '=' {
                            let mut taken = false;
                            let (_, next) = s.split_once('=').unwrap();
                            let mut nextfn = || {
                                taken = true;
                                Ok(next.into())
                            };
                            parser.short(c, true, &mut nextfn)?;

                            if !taken {
                                return Err(format!(
                                    "Flag '{s}' was given argument '{next}' without using it"
                                )
                                .into());
                            }

                            break;
                        } else {
                            parser.short(c, false, &mut || {
                                Err(
                                    format!("Could not get argument for {c} while in short chain")
                                        .into(),
                                )
                            })?;
                        }
                    }
                    None => {
                        let mut nextfn = || match args.peek() {
                            Some(f) => {
                                if f.starts_with('-') {
                                    Err(format!("Expected value, got flag {f}").into())
                                } else {
                                    Ok(args.next().unwrap())
                                }
                            }
                            None => {
                                Err(format!("Expected value for {c} but no arguments were left")
                                    .into())
                            }
                        };
                        parser.short(c, true, &mut nextfn)?;
                    }
                }
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if parser.argument(&arg, &mut || args.next().ok_or("No argument next".into()))? {
                parser.subcommand(&arg, Box::new(args))?;
                return Ok(parser);
            };
        }
    }

    Ok(parser)
}
