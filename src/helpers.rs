/*
 * libssg
 *
 * Copyright 2020 Manos Pitsidianakis
 *
 * This file is part of libssg.
 *
 * libssg is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * libssg is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with libssg. If not, see <http://www.gnu.org/licenses/>.
 */

//! Helper functions.

use handlebars::{Context, Handlebars, Helper, JsonRender, Output, RenderContext, RenderError};

/// Include HTML string without escaping.
pub fn include_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let param = h
        .param(0)
        .ok_or(RenderError::new("Param 0 is required for format helper."))?;
    out.write(&param.value().render())?;
    Ok(())
}

/// Format timestamp to date with a chrono format string
/// Usage: `{{ date_fmt date "%Y-%m-%d" }}`
pub fn date_fmt(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    use chrono::TimeZone;
    let fmt_string = h
        .param(1)
        .ok_or(RenderError::new(
            "Format string as second parameter is required for date_fmt helper.",
        ))?
        .value()
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            RenderError::new(
                "Second parameter format string must be of type string for date_fmt helper.",
            )
        })?;

    let date_s: i64 = match h
        .param(0)
        .ok_or(RenderError::new(
            "Date as first parameter is required for date_fmt helper.",
        ))?
        .value()
    {
        serde_json::Value::String(s) => chrono::Local
            .datetime_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .timestamp(),
        serde_json::Value::Number(num) if num.as_i64().is_some() => num.as_i64().unwrap(),
        _ => panic!(),
    };
    let date = chrono::Local.timestamp(date_s, 0);
    out.write(&date.format(&fmt_string).to_string())?;
    Ok(())
}

/// Usage: `{{ url_prefix }}`
pub fn url_prefix(
     : &Helper,
    h: &Handlebars,
    c: &Context,
    rc: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    use chrono::TimeZone;
    let disable_escape = rc.is_disable_escape();
    rc.set_disable_escape(true);
    h.register_escape_fn(|s| s.to_string());

    out.write(&c.get("ROOT_PREFIX"))?;
    rc.set_disable_escape(disable_escape);
    h.unregister_escape_fn();
    Ok(())
}
