use crate::ast::{Level, Message, Timestamp};
use anyhow::Context;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_till1},
    character::complete::{digit1, space0, space1},
    combinator::{complete, map, map_res},
    error::{FromExternalError, ParseError},
    multi::fold_many0,
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

pub fn parse_message<'i, E>(i: &'i str) -> IResult<&'i str, Message<'i>, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, anyhow::Error>,
{
    let ts = map_res(
        tuple((digit1, tag(":"), digit1, tag(":"), digit1)),
        |(hh, _, mm, _, ss): (&str, &str, &str, &str, &str)| {
            let hour = hh.parse().context("invalid hour")?;
            let minute = mm.parse().context("invalid minute")?;
            let second = ss.parse().context("invalid second")?;
            Ok(Timestamp {
                hour,
                minute,
                second,
            })
        },
    );
    let level = alt((
        map(tag("TRACE"), |_| Level::Trace),
        map(tag("DEBUG"), |_| Level::Debug),
        map(tag("INFO"), |_| Level::Info),
        map(tag("ALERT"), |_| Level::Alert),
        map(tag("WARN"), |_| Level::Warn),
        map(tag("ERROR"), |_| Level::Error),
    ));
    let source = take_till1(|c: char| c == ']');
    let contents = take_till(|c: char| c == '\n');

    let header = delimited(
        tag("["),
        tuple((
            preceded(space0, ts),
            preceded(space1, level),
            preceded(space1, source),
        )),
        tag("]"),
    );
    let message = separated_pair(header, tag(" "), contents);

    map(
        message,
        |((timestamp, level, source), contents): ((Timestamp, Level, &str), &str)| Message {
            timestamp,
            level,
            source: source.into(),
            contents: contents.into(),
        },
    )(i)
}

pub fn parse_log<'i, E>(i: &'i str) -> IResult<&'i str, Vec<Message<'i>>, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, anyhow::Error>,
{
    enum ParsedLine<'i> {
        Start(Message<'i>),
        Continued(&'i str),
    }

    let parse_line_or_continuation = alt((
        map(parse_message, ParsedLine::Start),
        map(take_till(|c: char| c == '\n'), ParsedLine::Continued),
    ));
    let parse_log = fold_many0(
        terminated(parse_line_or_continuation, tag("\n")),
        || Ok(Vec::new()),
        |acc, cur| {
            let mut acc = acc?;
            match cur {
                ParsedLine::Start(message) => {
                    acc.push(message);
                    Ok(acc)
                }
                ParsedLine::Continued(continued_contents) => {
                    let mut last = acc.pop().context("no message to continue")?;
                    let mut contents = last.contents.into_owned();
                    contents.push('\n');
                    contents.push_str(continued_contents);
                    last.contents = contents.into();
                    acc.push(last);
                    Ok(acc)
                }
            }
        },
    );

    map_res(parse_log, |messages| messages)(i)
}

pub fn parse_log_complete<'i, E>(i: &'i str) -> IResult<&'i str, Vec<Message<'i>>, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, anyhow::Error>,
{
    complete(parse_log)(i)
}

pub fn parse<'i>(contents: &'i str) -> anyhow::Result<Vec<Message<'i>>> {
    let (_, messages) = parse_log_complete::<nom::error::Error<&'i str>>(contents)
        .map_err(|error| anyhow::anyhow!("{}", error))?;
    Ok(messages)
}
