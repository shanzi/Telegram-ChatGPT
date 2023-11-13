use anyhow::bail;
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, tag, take_till1, take_while1},
    character::{
        complete::{char, multispace0, none_of, space1},
        is_newline,
    },
    combinator::value,
    multi::many1,
    sequence::{delimited, preceded},
    IResult,
};

fn is_special_char(c: char) -> bool {
    matches!(
        c,
        '_' | '*'
            | '['
            | ']'
            | '('
            | ')'
            | '~'
            | '`'
            | '>'
            | '#'
            | '+'
            | '-'
            | '='
            | '|'
            | '{'
            | '}'
            | '.'
            | '!'
            | '\\'
    )
}

fn escaped_for_tg(text: impl AsRef<str>) -> String {
    let mut escaped_string = String::new();
    for c in text.as_ref().chars() {
        if is_special_char(c) {
            escaped_string.push('\\');
        }
        escaped_string.push(c);
    }

    escaped_string
}

fn parse_escaped_chars(paren: char) -> impl Fn(&str) -> IResult<&str, String> {
    move |input: &str| {
        escaped_transform(
            take_till1(|c| c == paren || c == '\\' || is_newline(c as u8)),
            '\\',
            alt((
                value("_", tag("\\_")),
                value("*", tag("\\*")),
                value("[", tag("\\[")),
                value("]", tag("\\]")),
                value("(", tag("\\(")),
                value(")", tag("\\)")),
                value("~", tag("\\~")),
                value("`", tag("\\`")),
                value(">", tag("\\>")),
                value("#", tag("\\#")),
                value("+", tag("\\+")),
                value("-", tag("\\-")),
                value("=", tag("\\=")),
                value("|", tag("\\|")),
                value("{", tag("\\{")),
                value("}", tag("\\}")),
                value(".", tag("\\.")),
                value("!", tag("\\!")),
                value("\\", tag("\\\\")),
            )),
        )(input)
    }
}

fn parse_emphasize(input: &str) -> IResult<&str, String> {
    let (input, content) = alt((
        delimited(tag("__"), parse_escaped_chars('_'), tag("__")),
        delimited(char('_'), parse_escaped_chars('_'), char('_')),
    ))(input)?;
    Ok((input, format!("_{}_", escaped_for_tg(content))))
}

fn parse_bold(input: &str) -> IResult<&str, String> {
    let (input, content) = alt((
        delimited(tag("**"), parse_escaped_chars('*'), tag("**")),
        delimited(char('*'), parse_escaped_chars('*'), char('*')),
    ))(input)?;
    Ok((input, format!("*{}*", escaped_for_tg(content))))
}

fn parse_code(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(char('`'), parse_escaped_chars('`'), char('`'))(input)?;
    Ok((input, format!("`{}`", escaped_for_tg(content))))
}

fn parse_link(input: &str) -> IResult<&str, String> {
    let (input, text) = delimited(char('['), parse_escaped_chars(']'), char(']'))(input)?;
    let (input, link) = delimited(char('('), parse_escaped_chars(')'), char(')'))(input)?;
    Ok((
        input,
        format!("[{}]({})", escaped_for_tg(text), escaped_for_tg(link)),
    ))
}

fn parse_plaintext(input: &str) -> IResult<&str, String> {
    let (input, content) = take_till1(|c| is_special_char(c) || is_newline(c as u8))(input)?;
    Ok((input, escaped_for_tg(content)))
}

fn parse_special_chars(input: &str) -> IResult<&str, String> {
    let (input, content) = take_while1(is_special_char)(input)?;
    Ok((input, escaped_for_tg(content)))
}

fn parse_header(input: &str) -> IResult<&str, String> {
    let (input, htag) = many1(char('#'))(input)?;
    let (input, _) = space1(input)?;
    let (input, title) = parse_plaintext(input)?;
    Ok((
        input,
        format!(
            "`{}` __{}__",
            escaped_for_tg(htag.into_iter().collect::<String>()),
            title
        ),
    ))
}

fn parse_paragraph(input: &str) -> IResult<&str, String> {
    let (input, components) = many1(alt((
        parse_emphasize,
        parse_bold,
        parse_code,
        parse_link,
        parse_plaintext,
        parse_special_chars,
    )))(input)?;
    Ok((input, components.join("")))
}

fn parse_codeblock(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(
        tag("```"),
        escaped_transform(none_of("\\`"), '\\', value('`', char('`'))),
        tag("```"),
    )(input)?;
    Ok((input, format!("```{}```", escaped_for_tg(content))))
}

pub fn parse_markdown(input: &str) -> IResult<&str, String> {
    let (input, lines) = many1(preceded(
        multispace0,
        alt((parse_codeblock, parse_header, parse_paragraph)),
    ))(input)?;
    Ok((input, lines.join("\n\n")))
}

pub fn escape_markdown(text: impl AsRef<str>) -> anyhow::Result<String> {
    if let Ok((_, content)) = parse_markdown(text.as_ref()) {
        Ok(content)
    } else {
        bail!("unable to correctly escape markdown")
    }
}
