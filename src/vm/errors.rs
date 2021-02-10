// Copyright (C) 2013-2020 Blockstack PBC, a public benefit corporation
// Copyright (C) 2020 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::error;
use std::fmt;

use rusqlite::Error as SqliteError;
use serde_json::Error as SerdeJSONErr;

use crate::util::errors::CostErrors;
use chainstate::burn::BlockHeaderHash;
pub use util::errors::CheckErrors;
use vm::contexts::StackTrace;
use vm::types::{TypeSignature, Value};

use crate::util::errors::DBError as DatabaseError;
use crate::util::errors::ParseError;
use crate::util::errors::{IncomparableError, InterpreterError, MarfError};
pub use crate::vm::analysis::check_argument_count;
pub use crate::vm::analysis::check_arguments_at_least;

pub type InterpreterResult<R> = Result<R, InterpreterError>;

#[cfg(test)]
impl From<InterpreterError> for () {
    fn from(err: InterpreterError) -> Self {}
}

#[cfg(test)]
mod test {
    use vm::execute;

    use crate::util::errors::ShortReturnType;

    use super::*;
    use util::errors::InterpreterFailureError;

    #[test]
    fn error_formats() {
        let t = "(/ 10 0)";
        let expected = "DivisionByZero
 Stack Trace: 
_native_:native_div
";

        assert_eq!(format!("{}", execute(t).unwrap_err()), expected);
    }

    #[test]
    fn equality() {
        assert_eq!(
            InterpreterError::ShortReturn(ShortReturnType::ExpectedValue(Value::Bool(true))),
            InterpreterError::ShortReturn(ShortReturnType::ExpectedValue(Value::Bool(true)))
        );
        assert_eq!(
            InterpreterError::Interpreter(InterpreterFailureError::InterpreterError(
                "".to_string()
            )),
            InterpreterError::Interpreter(InterpreterFailureError::InterpreterError(
                "".to_string()
            ))
        );
        assert!(
            InterpreterError::ShortReturn(ShortReturnType::ExpectedValue(Value::Bool(true)))
                != InterpreterError::Interpreter(InterpreterFailureError::InterpreterError(
                    "".to_string()
                ))
        );
    }
}
