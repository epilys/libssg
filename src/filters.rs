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

use minijinja::{
    value::{Value, ValueKind},
    Error, ErrorKind, State,
};

pub fn sort_by_key(_: &State, value: Value, attr: &str) -> Result<Value, Error> {
    if value.kind() != ValueKind::Seq {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "sort_by_key: value is not a list",
        ));
    }
    let mut values = value
        .try_iter()
        .map_err(|err| {
            Error::new(
                ErrorKind::InvalidOperation,
                "sort_by_key: cannot convert value to list",
            )
            .with_source(err)
        })?
        .collect::<Vec<Value>>();
    let _attrs = values
        .iter()
        .map(|el| el.get_attr(attr))
        .collect::<Result<Value, Error>>()
        .map_err(|err| {
            Error::new(
                ErrorKind::InvalidOperation,
                "sort_by_key: value is not a dictionary/map",
            )
            .with_source(err)
        })?;
    values.sort_by(|a, b| {
        a.get_attr(attr)
            .unwrap()
            .partial_cmp(&b.get_attr(attr).unwrap())
            .unwrap_or(std::cmp::Ordering::Less)
    });
    Ok(Value::from(values))
}
