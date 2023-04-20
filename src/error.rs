/*
 * This file is part of mailpot
 *
 * Copyright 2020 - Manos Pitsidianakis
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! Errors of this library.

pub use anyhow::Context;
pub use error_chain::ChainedError;

// Create the Error, ErrorKind, ResultExt, and Result types

error_chain::error_chain! {
   errors {
       /*
       /// Post rejected.
       PostRejected(reason: String) {
           description("Post rejected")
           display("Your post has been rejected: {}", reason)
       }

       /// An entry was not found in the database.
       NotFound(model: &'static str) {
           description("Not found")
           display("This {} is not present in the database.", model)
       }

       /// A request was invalid.
       InvalidRequest(reason: String) {
           description("List request is invalid")
           display("Your list request has been found invalid: {}.", reason)
       }

       /// An error happened and it was handled internally.
       Information(reason: String) {
           description("An error happened and it was handled internally.")
           display("{}.", reason)
       }

       /// An error that shouldn't happen and should be reported.
       Bug(reason: String) {
           description("An error that shouldn't happen and should be reported.")
           display("{}.", reason)
       }
       */
   }
   foreign_links {
       Logic(anyhow::Error) #[doc="Error returned from an external user initiated operation such as deserialization or I/O."];
       Io(::std::io::Error) #[doc="Error returned from internal I/O operations."];
       SerdeJson(serde_json::Error) #[doc="Error from deserializing JSON values."];
       Template(minijinja::Error) #[doc="Error returned from minijinja template engine."];
       StripPrefix(std::path::StripPrefixError) #[doc="Error returned from minijinja template engine."];
   }
}