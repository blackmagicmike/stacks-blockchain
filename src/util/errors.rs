use std::error::Error;
use std::{error, io};

use serde::export::fmt;

use chainstate::burn::BlockHeaderHash;
use chainstate::stacks::events::StacksTransactionEvent;
use chainstate::stacks::index::node;
use net::NeighborKey;
use vm::contexts::{AssetMap, GlobalContext, StackTrace};
use vm::costs::ExecutionCost;
use vm::diagnostic::Diagnostic;
use vm::representations::PreSymbolicExpression;
use vm::types::{TupleTypeSignature, TypeSignature};
use vm::{SymbolicExpression, Value, MAX_CALL_STACK_DEPTH};

#[derive(Debug)]
pub enum NetworkError {
    /// Failed to encode
    SerializeError(String),
    /// Failed to read
    ReadError(io::Error),
    /// Failed to decode
    DeserializeError(String),
    /// Failed to write
    WriteError(io::Error),
    /// Underflow -- not enough bytes to form the message
    UnderflowError(String),
    /// Overflow -- message too big
    OverflowError(String),
    /// Wrong protocol family
    WrongProtocolFamily,
    /// Array is too big
    ArrayTooLong,
    /// Receive timed out
    RecvTimeout,
    /// Error signing a message
    SigningError(String),
    /// Error verifying a message
    VerifyingError(String),
    /// Read stream is drained.  Try again
    TemporarilyDrained,
    /// Read stream has reached EOF (socket closed, end-of-file reached, etc.)
    PermanentlyDrained,
    /// Failed to read from the FS
    FilesystemError,
    /// Database error
    DBError(DBError),
    /// Socket mutex was poisoned
    SocketMutexPoisoned,
    /// Socket not instantiated
    SocketNotConnectedToPeer,
    /// Not connected to peer
    ConnectionBroken,
    /// Connection could not be (re-)established
    ConnectionError,
    /// Too many outgoing messages
    OutboxOverflow,
    /// Too many incoming messages
    InboxOverflow,
    /// Send error
    SendError(String),
    /// Recv error
    RecvError(String),
    /// Invalid message
    InvalidMessage,
    /// Invalid network handle
    InvalidHandle,
    /// Network handle is full
    FullHandle,
    /// Invalid handshake
    InvalidHandshake,
    /// Stale neighbor
    StaleNeighbor,
    /// No such neighbor
    NoSuchNeighbor,
    /// Failed to bind
    BindError,
    /// Failed to poll
    PollError,
    /// Failed to accept
    AcceptError,
    /// Failed to register socket with poller
    RegisterError,
    /// Failed to query socket metadata
    SocketError,
    /// server is not bound to a socket
    NotConnected,
    /// Remote peer is not connected
    PeerNotConnected,
    /// Too many peers
    TooManyPeers,
    /// Peer already connected
    AlreadyConnected(usize, NeighborKey),
    /// Message already in progress
    InProgress,
    /// Peer is denied
    Denied,
    /// Data URL is not known
    NoDataUrl,
    /// Peer is transmitting too fast
    PeerThrottled,
    /// Error resolving a DNS name
    LookupError(String),
    /// MARF error, percolated up from chainstate
    MARFError(MarfError),
    /// Clarity VM error, percolated up from chainstate
    ClarityError(ClarityError),
    /// Catch-all for chainstate errors that don't map cleanly into network errors
    ChainstateError(String),
    /// Catch-all for errors that a client should receive more information about
    ClientError(ClientError),
    /// Coordinator hung up
    CoordinatorClosed,
    /// view of state is stale (e.g. from the sortition db)
    StaleView,
    /// Tried to connect to myself
    ConnectionCycle,
    /// Requested data not found
    NotFoundError,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NetworkError::SerializeError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::DeserializeError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::ReadError(ref io) => fmt::Display::fmt(io, f),
            NetworkError::WriteError(ref io) => fmt::Display::fmt(io, f),
            NetworkError::UnderflowError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::OverflowError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::WrongProtocolFamily => write!(f, "Improper use of protocol family"),
            NetworkError::ArrayTooLong => write!(f, "Array too long"),
            NetworkError::RecvTimeout => write!(f, "Packet receive timeout"),
            NetworkError::SigningError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::VerifyingError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::TemporarilyDrained => {
                write!(f, "Temporarily out of bytes to read; try again later")
            }
            NetworkError::PermanentlyDrained => write!(f, "Out of bytes to read"),
            NetworkError::FilesystemError => write!(f, "Disk I/O error"),
            NetworkError::DBError(ref e) => fmt::Display::fmt(e, f),
            NetworkError::SocketMutexPoisoned => write!(f, "socket mutex was poisoned"),
            NetworkError::SocketNotConnectedToPeer => write!(f, "not connected to peer"),
            NetworkError::ConnectionBroken => write!(f, "connection to peer node is broken"),
            NetworkError::ConnectionError => {
                write!(f, "connection to peer could not be (re-)established")
            }
            NetworkError::OutboxOverflow => write!(f, "too many outgoing messages queued"),
            NetworkError::InboxOverflow => write!(f, "too many messages pending"),
            NetworkError::SendError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::RecvError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::InvalidMessage => {
                write!(f, "invalid message (malformed or bad signature)")
            }
            NetworkError::InvalidHandle => write!(f, "invalid network handle"),
            NetworkError::FullHandle => write!(f, "network handle is full and needs to be drained"),
            NetworkError::InvalidHandshake => write!(f, "invalid handshake from remote peer"),
            NetworkError::StaleNeighbor => write!(f, "neighbor is too far behind the chain tip"),
            NetworkError::NoSuchNeighbor => write!(f, "no such neighbor"),
            NetworkError::BindError => write!(f, "Failed to bind to the given address"),
            NetworkError::PollError => write!(f, "Failed to poll"),
            NetworkError::AcceptError => write!(f, "Failed to accept connection"),
            NetworkError::RegisterError => write!(f, "Failed to register socket with poller"),
            NetworkError::SocketError => write!(f, "Socket error"),
            NetworkError::NotConnected => write!(f, "Not connected to peer network"),
            NetworkError::PeerNotConnected => write!(f, "Remote peer is not connected to us"),
            NetworkError::TooManyPeers => write!(f, "Too many peer connections open"),
            NetworkError::AlreadyConnected(ref _id, ref _nk) => write!(f, "Peer already connected"),
            NetworkError::InProgress => write!(f, "Message already in progress"),
            NetworkError::Denied => write!(f, "Peer is denied"),
            NetworkError::NoDataUrl => write!(f, "No data URL available"),
            NetworkError::PeerThrottled => write!(f, "Peer is transmitting too fast"),
            NetworkError::LookupError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::ChainstateError(ref s) => fmt::Display::fmt(s, f),
            NetworkError::ClarityError(ref e) => fmt::Display::fmt(e, f),
            NetworkError::MARFError(ref e) => fmt::Display::fmt(e, f),
            NetworkError::ClientError(ref e) => write!(f, "ClientError: {}", e),
            NetworkError::CoordinatorClosed => write!(f, "Coordinator hung up"),
            NetworkError::StaleView => write!(f, "State view is stale"),
            NetworkError::ConnectionCycle => write!(f, "Tried to connect to myself"),
            NetworkError::NotFoundError => write!(f, "Requested data not found"),
        }
    }
}

impl error::Error for NetworkError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            NetworkError::SerializeError(ref _s) => None,
            NetworkError::ReadError(ref io) => Some(io),
            NetworkError::DeserializeError(ref _s) => None,
            NetworkError::WriteError(ref io) => Some(io),
            NetworkError::UnderflowError(ref _s) => None,
            NetworkError::OverflowError(ref _s) => None,
            NetworkError::WrongProtocolFamily => None,
            NetworkError::ArrayTooLong => None,
            NetworkError::RecvTimeout => None,
            NetworkError::SigningError(ref _s) => None,
            NetworkError::VerifyingError(ref _s) => None,
            NetworkError::TemporarilyDrained => None,
            NetworkError::PermanentlyDrained => None,
            NetworkError::FilesystemError => None,
            NetworkError::DBError(ref e) => Some(e),
            NetworkError::SocketMutexPoisoned => None,
            NetworkError::SocketNotConnectedToPeer => None,
            NetworkError::ConnectionBroken => None,
            NetworkError::ConnectionError => None,
            NetworkError::OutboxOverflow => None,
            NetworkError::InboxOverflow => None,
            NetworkError::SendError(ref _s) => None,
            NetworkError::RecvError(ref _s) => None,
            NetworkError::InvalidMessage => None,
            NetworkError::InvalidHandle => None,
            NetworkError::FullHandle => None,
            NetworkError::InvalidHandshake => None,
            NetworkError::StaleNeighbor => None,
            NetworkError::NoSuchNeighbor => None,
            NetworkError::BindError => None,
            NetworkError::PollError => None,
            NetworkError::AcceptError => None,
            NetworkError::RegisterError => None,
            NetworkError::SocketError => None,
            NetworkError::NotConnected => None,
            NetworkError::PeerNotConnected => None,
            NetworkError::TooManyPeers => None,
            NetworkError::AlreadyConnected(ref _id, ref _nk) => None,
            NetworkError::InProgress => None,
            NetworkError::Denied => None,
            NetworkError::NoDataUrl => None,
            NetworkError::PeerThrottled => None,
            NetworkError::LookupError(ref _s) => None,
            NetworkError::ChainstateError(ref _s) => None,
            NetworkError::ClientError(ref e) => Some(e),
            NetworkError::ClarityError(ref e) => Some(e),
            NetworkError::MARFError(ref e) => Some(e),
            NetworkError::CoordinatorClosed => None,
            NetworkError::StaleView => None,
            NetworkError::ConnectionCycle => None,
            NetworkError::NotFoundError => None,
        }
    }
}

#[cfg(test)]
impl PartialEq for NetworkError {
    /// (make I/O errors comparable for testing purposes)
    fn eq(&self, other: &Self) -> bool {
        let s1 = format!("{:?}", self);
        let s2 = format!("{:?}", other);
        s1 == s2
    }
}

#[derive(Debug)]
pub enum DBError {
    /// Not implemented
    NotImplemented,
    /// Database doesn't exist
    NoDBError,
    /// Read-only and tried to write
    ReadOnly,
    /// Type error -- can't represent the given data in the database
    TypeError,
    /// Database is corrupt -- we got data that shouldn't be there, or didn't get data when we
    /// should have
    Corruption,
    /// Serialization error -- can't serialize data
    SerializationError(serde_json::Error),
    /// Parse error -- failed to load data we stored directly
    ParseError,
    /// Operation would overflow
    Overflow,
    /// Data not found
    NotFoundError,
    /// Data already exists
    ExistsError,
    /// Data corresponds to a non-canonical PoX sortition
    InvalidPoxSortition,
    /// Sqlite3 error
    SqliteError(rusqlite::Error),
    /// I/O error
    IOError(io::Error),
    /// MARF index error
    IndexError(MarfError),
    /// Other error
    Other(String),
}

impl fmt::Display for DBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DBError::NotImplemented => write!(f, "Not implemented"),
            DBError::NoDBError => write!(f, "Database does not exist"),
            DBError::ReadOnly => write!(f, "Database is opened read-only"),
            DBError::TypeError => write!(f, "Invalid or unrepresentable database type"),
            DBError::Corruption => write!(f, "Database is corrupt"),
            DBError::SerializationError(ref e) => fmt::Display::fmt(e, f),
            DBError::ParseError => write!(f, "Parse error"),
            DBError::Overflow => write!(f, "Numeric overflow"),
            DBError::NotFoundError => write!(f, "Not found"),
            DBError::ExistsError => write!(f, "Already exists"),
            DBError::InvalidPoxSortition => write!(f, "Invalid PoX sortition"),
            DBError::IOError(ref e) => fmt::Display::fmt(e, f),
            DBError::SqliteError(ref e) => fmt::Display::fmt(e, f),
            DBError::IndexError(ref e) => fmt::Display::fmt(e, f),
            DBError::Other(ref s) => fmt::Display::fmt(s, f),
        }
    }
}

impl error::Error for DBError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            DBError::NotImplemented => None,
            DBError::NoDBError => None,
            DBError::ReadOnly => None,
            DBError::TypeError => None,
            DBError::Corruption => None,
            DBError::SerializationError(ref e) => Some(e),
            DBError::ParseError => None,
            DBError::Overflow => None,
            DBError::NotFoundError => None,
            DBError::ExistsError => None,
            DBError::InvalidPoxSortition => None,
            DBError::SqliteError(ref e) => Some(e),
            DBError::IOError(ref e) => Some(e),
            DBError::IndexError(ref e) => Some(e),
            DBError::Other(ref _s) => None,
        }
    }
}

impl From<rusqlite::Error> for DBError {
    fn from(e: rusqlite::Error) -> DBError {
        DBError::SqliteError(e)
    }
}

impl From<MarfError> for DBError {
    fn from(e: MarfError) -> DBError {
        DBError::IndexError(e)
    }
}

#[derive(Debug)]
pub enum ClarityError {
    Analysis(CheckError),
    Parse(ParseError),
    Interpreter(InterpreterError),
    BadTransaction(String),
    CostError(ExecutionCost, ExecutionCost),
    AbortedByCallback(Option<Value>, AssetMap, Vec<StacksTransactionEvent>),
}

impl From<CheckError> for ClarityError {
    fn from(e: CheckError) -> Self {
        match e.err {
            CheckErrors::CostOverflow => {
                ClarityError::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            CheckErrors::CostBalanceExceeded(a, b) => ClarityError::CostError(a, b),
            CheckErrors::MemoryBalanceExceeded(_a, _b) => {
                ClarityError::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            _ => ClarityError::Analysis(e),
        }
    }
}

impl From<InterpreterError> for ClarityError {
    fn from(e: InterpreterError) -> Self {
        match &e {
            InterpreterError::Unchecked(CheckErrors::CostBalanceExceeded(a, b)) => {
                ClarityError::CostError(a.clone(), b.clone())
            }
            InterpreterError::Unchecked(CheckErrors::CostOverflow) => {
                ClarityError::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            _ => ClarityError::Interpreter(e),
        }
    }
}

impl From<ParseError> for ClarityError {
    fn from(e: ParseError) -> Self {
        match e.err {
            ParseErrors::CostOverflow => {
                ClarityError::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            ParseErrors::CostBalanceExceeded(a, b) => ClarityError::CostError(a, b),
            ParseErrors::MemoryBalanceExceeded(_a, _b) => {
                ClarityError::CostError(ExecutionCost::max_value(), ExecutionCost::max_value())
            }
            _ => ClarityError::Parse(e),
        }
    }
}

impl From<ChainstateError> for ClarityError {
    fn from(e: ChainstateError) -> Self {
        match e {
            ChainstateError::InvalidStacksTransaction(msg, _) => ClarityError::BadTransaction(msg),
            ChainstateError::CostOverflowError(_, after, budget) => {
                ClarityError::CostError(after, budget)
            }
            ChainstateError::ClarityError(x) => x,
            x => ClarityError::BadTransaction(format!("{:?}", &x)),
        }
    }
}

impl fmt::Display for ClarityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ClarityError::CostError(ref a, ref b) => {
                write!(f, "Cost Error: {} cost exceeded budget of {} cost", a, b)
            }
            ClarityError::Analysis(ref e) => fmt::Display::fmt(e, f),
            ClarityError::Parse(ref e) => fmt::Display::fmt(e, f),
            ClarityError::AbortedByCallback(..) => write!(f, "Post condition aborted transaction"),
            ClarityError::Interpreter(ref e) => fmt::Display::fmt(e, f),
            ClarityError::BadTransaction(ref s) => fmt::Display::fmt(s, f),
        }
    }
}

impl error::Error for ClarityError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            ClarityError::CostError(ref _a, ref _b) => None,
            ClarityError::AbortedByCallback(..) => None,
            ClarityError::Analysis(ref e) => Some(e),
            ClarityError::Parse(ref e) => Some(e),
            ClarityError::Interpreter(ref e) => Some(e),
            ClarityError::BadTransaction(ref _s) => None,
        }
    }
}

#[derive(Debug)]
pub enum InterpreterError {
    /// UncheckedErrors are errors that *should* be caught by the
    ///   TypeChecker and other check passes. Test executions may
    ///   trigger these errors.
    Unchecked(CheckErrors),
    Interpreter(InterpreterFailureError),
    Runtime(RuntimeErrorType, Option<StackTrace>),
    ShortReturn(ShortReturnType),
}

/// InterpreterFailureErrors are errors that *should never* occur.
/// Test executions may trigger these errors.
#[derive(Debug, PartialEq)]
pub enum InterpreterFailureError {
    BadSender(Value),
    BadSymbolicRepresentation(String),
    InterpreterError(String),
    UninitializedPersistedVariable,
    FailedToConstructAssetTable,
    FailedToConstructEventBatch,
    SqliteError(IncomparableError<rusqlite::Error>),
    BadFileName,
    FailedToCreateDataDirectory,
    MarfFailure(IncomparableError<MarfError>),
    FailureConstructingTupleWithType,
    FailureConstructingListWithType,
    InsufficientBalance,
    CostContractLoadFailure,
    DBError(IncomparableError<DBError>),
}

impl PartialEq<InterpreterError> for InterpreterError {
    fn eq(&self, other: &InterpreterError) -> bool {
        match (self, other) {
            (InterpreterError::Runtime(x, _), InterpreterError::Runtime(y, _)) => x == y,
            (InterpreterError::Unchecked(x), InterpreterError::Unchecked(y)) => x == y,
            (InterpreterError::ShortReturn(x), InterpreterError::ShortReturn(y)) => x == y,
            (InterpreterError::Interpreter(x), InterpreterError::Interpreter(y)) => x == y,
            _ => false,
        }
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InterpreterError::Runtime(ref err, ref stack) => {
                match err {
                    _ => write!(f, "{}", err),
                }?;

                if let Some(ref stack_trace) = stack {
                    write!(f, "\n Stack Trace: \n")?;
                    for item in stack_trace.iter() {
                        write!(f, "{}\n", item)?;
                    }
                }
                Ok(())
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl error::Error for InterpreterError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<CostErrors> for InterpreterError {
    fn from(err: CostErrors) -> Self {
        InterpreterError::from(CheckErrors::from(err))
    }
}

impl From<ParseError> for InterpreterError {
    fn from(err: ParseError) -> Self {
        InterpreterError::from(RuntimeErrorType::ASTError(err))
    }
}

impl From<serde_json::Error> for InterpreterError {
    fn from(err: serde_json::Error) -> Self {
        InterpreterError::from(RuntimeErrorType::JSONParseError(IncomparableError { err }))
    }
}

impl From<RuntimeErrorType> for InterpreterError {
    fn from(err: RuntimeErrorType) -> Self {
        InterpreterError::Runtime(err, None)
    }
}

impl From<CheckErrors> for InterpreterError {
    fn from(err: CheckErrors) -> Self {
        InterpreterError::Unchecked(err)
    }
}

impl From<ShortReturnType> for InterpreterError {
    fn from(err: ShortReturnType) -> Self {
        InterpreterError::ShortReturn(err)
    }
}

impl From<InterpreterFailureError> for InterpreterError {
    fn from(err: InterpreterFailureError) -> Self {
        InterpreterError::Interpreter(err)
    }
}

#[derive(Debug)]
pub enum ChainstateError {
    InvalidFee,
    InvalidStacksBlock(String),
    InvalidStacksMicroblock(String, BlockHeaderHash),
    InvalidStacksTransaction(String, bool),
    PostConditionFailed(String),
    NoSuchBlockError,
    InvalidChainstateDB,
    BlockTooBigError,
    BlockCostExceeded,
    NoTransactionsToMine,
    MicroblockStreamTooLongError,
    IncompatibleSpendingConditionError,
    CostOverflowError(ExecutionCost, ExecutionCost, ExecutionCost),
    ClarityError(ClarityError),
    DBError(DBError),
    NetError(NetworkError),
    MARFError(MarfError),
    ReadError(io::Error),
    WriteError(io::Error),
    MemPoolError(String),
    PoxAlreadyLocked,
    PoxInsufficientBalance,
    PoxNoRewardCycle,
}

impl From<MarfError> for ChainstateError {
    fn from(e: MarfError) -> ChainstateError {
        ChainstateError::MARFError(e)
    }
}

impl From<ClarityError> for ChainstateError {
    fn from(e: ClarityError) -> ChainstateError {
        ChainstateError::ClarityError(e)
    }
}

impl From<NetworkError> for ChainstateError {
    fn from(e: NetworkError) -> ChainstateError {
        ChainstateError::NetError(e)
    }
}

impl fmt::Display for ChainstateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ChainstateError::InvalidFee => write!(f, "Invalid fee"),
            ChainstateError::InvalidStacksBlock(ref s) => fmt::Display::fmt(s, f),
            ChainstateError::InvalidStacksMicroblock(ref s, _) => fmt::Display::fmt(s, f),
            ChainstateError::InvalidStacksTransaction(ref s, _) => fmt::Display::fmt(s, f),
            ChainstateError::PostConditionFailed(ref s) => fmt::Display::fmt(s, f),
            ChainstateError::NoSuchBlockError => write!(f, "No such Stacks block"),
            ChainstateError::InvalidChainstateDB => write!(f, "Invalid chainstate database"),
            ChainstateError::BlockTooBigError => write!(f, "Too much data in block"),
            ChainstateError::BlockCostExceeded => write!(f, "Block execution budget exceeded"),
            ChainstateError::MicroblockStreamTooLongError => {
                write!(f, "Too many microblocks in stream")
            }
            ChainstateError::IncompatibleSpendingConditionError => {
                write!(f, "Spending condition is incompatible with this operation")
            }
            ChainstateError::CostOverflowError(ref c1, ref c2, ref c3) => write!(
                f,
                "{}",
                &format!(
                    "Cost overflow: before={:?}, after={:?}, budget={:?}",
                    c1, c2, c3
                )
            ),
            ChainstateError::ClarityError(ref e) => fmt::Display::fmt(e, f),
            ChainstateError::DBError(ref e) => fmt::Display::fmt(e, f),
            ChainstateError::NetError(ref e) => fmt::Display::fmt(e, f),
            ChainstateError::MARFError(ref e) => fmt::Display::fmt(e, f),
            ChainstateError::ReadError(ref e) => fmt::Display::fmt(e, f),
            ChainstateError::WriteError(ref e) => fmt::Display::fmt(e, f),
            ChainstateError::MemPoolError(ref s) => fmt::Display::fmt(s, f),
            ChainstateError::NoTransactionsToMine => write!(f, "No transactions to mine"),
            ChainstateError::PoxAlreadyLocked => {
                write!(f, "Account has already locked STX for PoX")
            }
            ChainstateError::PoxInsufficientBalance => write!(f, "Not enough STX to lock"),
            ChainstateError::PoxNoRewardCycle => write!(f, "No such reward cycle"),
        }
    }
}

impl error::Error for ChainstateError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            ChainstateError::InvalidFee => None,
            ChainstateError::InvalidStacksBlock(ref _s) => None,
            ChainstateError::InvalidStacksMicroblock(ref _s, ref _h) => None,
            ChainstateError::InvalidStacksTransaction(ref _s, _q) => None,
            ChainstateError::PostConditionFailed(ref _s) => None,
            ChainstateError::NoSuchBlockError => None,
            ChainstateError::InvalidChainstateDB => None,
            ChainstateError::BlockTooBigError => None,
            ChainstateError::BlockCostExceeded => None,
            ChainstateError::MicroblockStreamTooLongError => None,
            ChainstateError::IncompatibleSpendingConditionError => None,
            ChainstateError::CostOverflowError(..) => None,
            ChainstateError::ClarityError(ref e) => Some(e),
            ChainstateError::DBError(ref e) => Some(e),
            ChainstateError::NetError(ref e) => Some(e),
            ChainstateError::MARFError(ref e) => Some(e),
            ChainstateError::ReadError(ref e) => Some(e),
            ChainstateError::WriteError(ref e) => Some(e),
            ChainstateError::MemPoolError(ref _s) => None,
            ChainstateError::NoTransactionsToMine => None,
            ChainstateError::PoxAlreadyLocked => None,
            ChainstateError::PoxInsufficientBalance => None,
            ChainstateError::PoxNoRewardCycle => None,
        }
    }
}

impl ChainstateError {
    fn name(&self) -> &'static str {
        match self {
            ChainstateError::InvalidFee => "InvalidFee",
            ChainstateError::InvalidStacksBlock(ref _s) => "InvalidStacksBlock",
            ChainstateError::InvalidStacksMicroblock(ref _s, ref _h) => "InvalidStacksMicroblock",
            ChainstateError::InvalidStacksTransaction(ref _s, _q) => "InvalidStacksTransaction",
            ChainstateError::PostConditionFailed(ref _s) => "PostConditionFailed",
            ChainstateError::NoSuchBlockError => "NoSuchBlockError",
            ChainstateError::InvalidChainstateDB => "InvalidChainstateDB",
            ChainstateError::BlockTooBigError => "BlockTooBigError",
            ChainstateError::BlockCostExceeded => "BlockCostExceeded",
            ChainstateError::MicroblockStreamTooLongError => "MicroblockStreamTooLongError",
            ChainstateError::IncompatibleSpendingConditionError => {
                "IncompatibleSpendingConditionError"
            }
            ChainstateError::CostOverflowError(..) => "CostOverflowError",
            ChainstateError::ClarityError(ref _e) => "ClarityError",
            ChainstateError::DBError(ref _e) => "DBError",
            ChainstateError::NetError(ref _e) => "NetError",
            ChainstateError::MARFError(ref _e) => "MARFError",
            ChainstateError::ReadError(ref _e) => "ReadError",
            ChainstateError::WriteError(ref _e) => "WriteError",
            ChainstateError::MemPoolError(ref _s) => "MemPoolError",
            ChainstateError::NoTransactionsToMine => "NoTransactionsToMine",
            ChainstateError::PoxAlreadyLocked => "PoxAlreadyLocked",
            ChainstateError::PoxInsufficientBalance => "PoxInsufficientBalance",
            ChainstateError::PoxNoRewardCycle => "PoxNoRewardCycle",
        }
    }

    pub fn into_json(&self) -> serde_json::Value {
        let reason_code = self.name();
        let reason_data = format!("{:?}", &self);
        let result = json!({
            "error": "chainstate error",
            "reason": reason_code,
            "reason_data": reason_data
        });
        result
    }
}

impl From<rusqlite::Error> for ChainstateError {
    fn from(e: rusqlite::Error) -> ChainstateError {
        ChainstateError::DBError(DBError::SqliteError(e))
    }
}

impl From<DBError> for ChainstateError {
    fn from(e: DBError) -> ChainstateError {
        ChainstateError::DBError(e)
    }
}

impl From<InterpreterError> for ChainstateError {
    fn from(e: InterpreterError) -> ChainstateError {
        ChainstateError::ClarityError(ClarityError::Interpreter(e))
    }
}

impl ChainstateError {
    pub fn from_cost_error(
        err: CostErrors,
        cost_before: ExecutionCost,
        context: &GlobalContext,
    ) -> ChainstateError {
        match err {
            CostErrors::CostBalanceExceeded(used, budget) => {
                ChainstateError::CostOverflowError(cost_before, used, budget)
            }
            _ => {
                let cur_cost = context.cost_track.get_total();
                let budget = context.cost_track.get_limit();
                ChainstateError::CostOverflowError(cost_before, cur_cost, budget)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckError {
    pub err: CheckErrors,
    pub expressions: Option<Vec<SymbolicExpression>>,
    pub diagnostic: Diagnostic,
}

impl CheckError {
    pub fn new(err: CheckErrors) -> CheckError {
        let diagnostic = Diagnostic::err(&err);
        CheckError {
            err,
            expressions: None,
            diagnostic,
        }
    }

    pub fn has_expression(&self) -> bool {
        self.expressions.is_some()
    }

    pub fn set_expression(&mut self, expr: &SymbolicExpression) {
        self.diagnostic.spans = vec![expr.span.clone()];
        self.expressions.replace(vec![expr.clone()]);
    }

    pub fn set_expressions(&mut self, exprs: &[SymbolicExpression]) {
        self.diagnostic.spans = exprs.iter().map(|e| e.span.clone()).collect();
        self.expressions.replace(exprs.clone().to_vec());
    }
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.err {
            _ => write!(f, "{}", self.err),
        }?;

        if let Some(ref e) = self.expressions {
            write!(f, "\nNear:\n{:?}", e)?;
        }

        Ok(())
    }
}

impl From<CostErrors> for CheckError {
    fn from(err: CostErrors) -> Self {
        CheckError::from(CheckErrors::from(err))
    }
}

impl error::Error for CheckError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<CheckErrors> for CheckError {
    fn from(err: CheckErrors) -> Self {
        CheckError::new(err)
    }
}

pub trait DiagnosableError {
    fn message(&self) -> String;
    fn suggestion(&self) -> Option<String>;
}

#[derive(Debug, PartialEq)]
pub enum CheckErrors {
    // cost checker errors
    CostOverflow,
    CostBalanceExceeded(ExecutionCost, ExecutionCost),
    MemoryBalanceExceeded(u64, u64),
    CostComputationFailed(String),

    ValueTooLarge,
    ValueOutOfBounds,
    TypeSignatureTooDeep,
    ExpectedName,

    // match errors
    BadMatchOptionSyntax(Box<CheckErrors>),
    BadMatchResponseSyntax(Box<CheckErrors>),
    BadMatchInput(TypeSignature),

    // list typing errors
    UnknownListConstructionFailure,
    ListTypesMustMatch,
    ConstructedListTooLarge,

    // simple type expectation mismatch
    TypeError(TypeSignature, TypeSignature),
    TypeLiteralError(TypeSignature, TypeSignature),
    TypeValueError(TypeSignature, Value),

    NoSuperType(TypeSignature, TypeSignature),
    InvalidTypeDescription,
    UnknownTypeName(String),

    // union type mismatch
    UnionTypeError(Vec<TypeSignature>, TypeSignature),
    UnionTypeValueError(Vec<TypeSignature>, Value),

    ExpectedLiteral,
    ExpectedOptionalType(TypeSignature),
    ExpectedResponseType(TypeSignature),
    ExpectedOptionalOrResponseType(TypeSignature),
    ExpectedOptionalValue(Value),
    ExpectedResponseValue(Value),
    ExpectedOptionalOrResponseValue(Value),
    CouldNotDetermineResponseOkType,
    CouldNotDetermineResponseErrType,
    UncheckedIntermediaryResponses,

    CouldNotDetermineMatchTypes,

    // Checker runtime failures
    TypeAlreadyAnnotatedFailure,
    TypeAnnotationExpectedFailure,
    CheckerImplementationFailure,

    // Assets
    BadTokenName,
    DefineFTBadSignature,
    DefineNFTBadSignature,
    NoSuchNFT(String),
    NoSuchFT(String),

    BadTransferSTXArguments,
    BadTransferFTArguments,
    BadTransferNFTArguments,
    BadMintFTArguments,
    BadBurnFTArguments,

    // tuples
    BadTupleFieldName,
    ExpectedTuple(TypeSignature),
    NoSuchTupleField(String, TupleTypeSignature),
    EmptyTuplesNotAllowed,
    BadTupleConstruction,
    TupleExpectsPairs,

    // variables
    NoSuchDataVariable(String),

    // data map
    BadMapName,
    NoSuchMap(String),

    // defines
    DefineFunctionBadSignature,
    BadFunctionName,
    BadMapTypeDefinition,
    PublicFunctionMustReturnResponse(TypeSignature),
    DefineVariableBadSignature,
    ReturnTypesMustMatch(TypeSignature, TypeSignature),

    CircularReference(Vec<String>),

    // contract-call errors
    NoSuchContract(String),
    NoSuchPublicFunction(String, String),
    PublicFunctionNotReadOnly(String, String),
    ContractAlreadyExists(String),
    ContractCallExpectName,

    // get-block-info? errors
    NoSuchBlockInfoProperty(String),
    GetBlockInfoExpectPropertyName,

    NameAlreadyUsed(String),

    // expect a function, or applying a function to a list
    NonFunctionApplication,
    ExpectedListApplication,
    ExpectedSequence(TypeSignature),
    MaxLengthOverflow,

    // let syntax
    BadLetSyntax,

    // generic binding syntax
    BadSyntaxBinding,
    BadSyntaxExpectedListOfPairs,

    MaxContextDepthReached,
    UndefinedFunction(String),
    UndefinedVariable(String),

    // argument counts
    RequiresAtLeastArguments(usize, usize),
    IncorrectArgumentCount(usize, usize),
    IfArmsMustMatch(TypeSignature, TypeSignature),
    MatchArmsMustMatch(TypeSignature, TypeSignature),
    DefaultTypesMustMatch(TypeSignature, TypeSignature),
    TooManyExpressions,
    IllegalOrUnknownFunctionApplication(String),
    UnknownFunction(String),

    // traits
    TraitReferenceUnknown(String),
    TraitMethodUnknown(String, String),
    ExpectedTraitIdentifier,
    ImportTraitBadSignature,
    TraitReferenceNotAllowed,
    BadTraitImplementation(String, String),
    DefineTraitBadSignature,
    UnexpectedTraitOrFieldReference,
    TraitBasedContractCallInReadOnly,
    ContractOfExpectsTrait,

    // strings
    InvalidCharactersDetected,

    // secp256k1 signature
    InvalidSecp65k1Signature,

    WriteAttemptedInReadOnly,
    AtBlockClosureMustBeReadOnly,
}

impl fmt::Display for CheckErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<CostErrors> for CheckErrors {
    fn from(err: CostErrors) -> Self {
        match err {
            CostErrors::CostOverflow => CheckErrors::CostOverflow,
            CostErrors::CostBalanceExceeded(a, b) => CheckErrors::CostBalanceExceeded(a, b),
            CostErrors::MemoryBalanceExceeded(a, b) => CheckErrors::MemoryBalanceExceeded(a, b),
            CostErrors::CostComputationFailed(s) => CheckErrors::CostComputationFailed(s),
            CostErrors::CostContractLoadFailure => {
                CheckErrors::CostComputationFailed("Failed to load cost contract".into())
            }
        }
    }
}

impl error::Error for CheckErrors {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

fn formatted_expected_types(expected_types: &Vec<TypeSignature>) -> String {
    let mut expected_types_joined = format!("'{}'", expected_types[0]);

    if expected_types.len() > 2 {
        for expected_type in expected_types[1..expected_types.len() - 1].into_iter() {
            expected_types_joined.push_str(&format!(", '{}'", expected_type));
        }
    }
    expected_types_joined.push_str(&format!(
        " or '{}'",
        expected_types[expected_types.len() - 1]
    ));
    expected_types_joined
}

impl DiagnosableError for CheckErrors {
    fn message(&self) -> String {
        match &self {
            CheckErrors::ExpectedLiteral => "expected a literal argument".into(),
            CheckErrors::BadMatchOptionSyntax(source) =>
                format!("match on a optional type uses the following syntax: (match input some-name if-some-expression if-none-expression). Caused by: {}",
                        source.message()),
            CheckErrors::BadMatchResponseSyntax(source) =>
                format!("match on a result type uses the following syntax: (match input ok-name if-ok-expression err-name if-err-expression). Caused by: {}",
                        source.message()),
            CheckErrors::BadMatchInput(t) =>
                format!("match requires an input of either a response or optional, found input: '{}'", t),
            CheckErrors::TypeAnnotationExpectedFailure => "analysis expected type to already be annotated for expression".into(),
            CheckErrors::CostOverflow => "contract execution cost overflowed cost counter".into(),
            CheckErrors::CostBalanceExceeded(a, b) => format!("contract execution cost exceeded budget: {:?} > {:?}", a, b),
            CheckErrors::MemoryBalanceExceeded(a, b) => format!("contract execution cost exceeded memory budget: {:?} > {:?}", a, b),
            CheckErrors::InvalidTypeDescription => "supplied type description is invalid".into(),
            CheckErrors::EmptyTuplesNotAllowed => "tuple common may not be empty".into(),
            CheckErrors::BadSyntaxExpectedListOfPairs => "bad syntax: function expects a list of pairs to bind names, e.g., ((name-0 a) (name-1 b) ...)".into(),
            CheckErrors::UnknownTypeName(name) => format!("failed to parse type: '{}'", name),
            CheckErrors::ValueTooLarge => format!("created a type which was greater than maximum allowed value size"),
            CheckErrors::ValueOutOfBounds => format!("created a type which value size was out of defined bounds"),
            CheckErrors::TypeSignatureTooDeep => "created a type which was deeper than maximum allowed type depth".into(),
            CheckErrors::ExpectedName => format!("expected a name argument to this function"),
            CheckErrors::NoSuperType(a, b) => format!("unable to create a supertype for the two common: '{}' and '{}'", a, b),
            CheckErrors::UnknownListConstructionFailure => format!("invalid syntax for list definition"),
            CheckErrors::ListTypesMustMatch => format!("expecting elements of same type in a list"),
            CheckErrors::ConstructedListTooLarge => format!("reached limit of elements in a list"),
            CheckErrors::TypeError(expected_type, found_type) => format!("expecting expression of type '{}', found '{}'", expected_type, found_type),
            CheckErrors::TypeLiteralError(expected_type, found_type) => format!("expecting a literal of type '{}', found '{}'", expected_type, found_type),
            CheckErrors::TypeValueError(expected_type, found_value) => format!("expecting expression of type '{}', found '{}'", expected_type, found_value),
            CheckErrors::UnionTypeError(expected_types, found_type) => format!("expecting expression of type {}, found '{}'", formatted_expected_types(expected_types), found_type),
            CheckErrors::UnionTypeValueError(expected_types, found_type) => format!("expecting expression of type {}, found '{}'", formatted_expected_types(expected_types), found_type),
            CheckErrors::ExpectedOptionalType(found_type) => format!("expecting expression of type 'optional', found '{}'", found_type),
            CheckErrors::ExpectedOptionalOrResponseType(found_type) => format!("expecting expression of type 'optional' or 'response', found '{}'", found_type),
            CheckErrors::ExpectedOptionalOrResponseValue(found_type) =>  format!("expecting expression of type 'optional' or 'response', found '{}'", found_type),
            CheckErrors::ExpectedResponseType(found_type) => format!("expecting expression of type 'response', found '{}'", found_type),
            CheckErrors::ExpectedOptionalValue(found_type) => format!("expecting expression of type 'optional', found '{}'", found_type),
            CheckErrors::ExpectedResponseValue(found_type) => format!("expecting expression of type 'response', found '{}'", found_type),
            CheckErrors::CouldNotDetermineResponseOkType => format!("attempted to obtain 'ok' value from response, but 'ok' type is indeterminate"),
            CheckErrors::CouldNotDetermineResponseErrType => format!("attempted to obtain 'err' value from response, but 'err' type is indeterminate"),
            CheckErrors::CouldNotDetermineMatchTypes => format!("attempted to match on an (optional) or (response) type where either the some, ok, or err type is indeterminate. you may wish to use unwrap-panic or unwrap-err-panic instead."),
            CheckErrors::BadTupleFieldName => format!("invalid tuple field name"),
            CheckErrors::ExpectedTuple(type_signature) => format!("expecting tuple, found '{}'", type_signature),
            CheckErrors::NoSuchTupleField(field_name, tuple_signature) => format!("cannot find field '{}' in tuple '{}'", field_name, tuple_signature),
            CheckErrors::BadTupleConstruction => format!("invalid tuple syntax, expecting list of pair"),
            CheckErrors::TupleExpectsPairs => format!("invalid tuple syntax, expecting pair"),
            CheckErrors::NoSuchDataVariable(var_name) => format!("use of unresolved persisted variable '{}'", var_name),
            CheckErrors::BadTransferSTXArguments => format!("STX transfer expects an int amount, from principal, to principal"),
            CheckErrors::BadTransferFTArguments => format!("transfer expects an int amount, from principal, to principal"),
            CheckErrors::BadTransferNFTArguments => format!("transfer expects an asset, from principal, to principal"),
            CheckErrors::BadMintFTArguments => format!("mint expects a uint amount and from principal"),
            CheckErrors::BadBurnFTArguments => format!("burn expects a uint amount and from principal"),
            CheckErrors::BadMapName => format!("invalid map name"),
            CheckErrors::NoSuchMap(map_name) => format!("use of unresolved map '{}'", map_name),
            CheckErrors::DefineFunctionBadSignature => format!("invalid function definition"),
            CheckErrors::BadFunctionName => format!("invalid function name"),
            CheckErrors::BadMapTypeDefinition => format!("invalid map definition"),
            CheckErrors::PublicFunctionMustReturnResponse(found_type) => format!("public functions must return an expression of type 'response', found '{}'", found_type),
            CheckErrors::DefineVariableBadSignature => format!("invalid variable definition"),
            CheckErrors::ReturnTypesMustMatch(type_1, type_2) => format!("detected two execution paths, returning two different expression common (got '{}' and '{}')", type_1, type_2),
            CheckErrors::NoSuchContract(contract_identifier) => format!("use of unresolved contract '{}'", contract_identifier),
            CheckErrors::NoSuchPublicFunction(contract_identifier, function_name) => format!("contract '{}' has no public function '{}'", contract_identifier, function_name),
            CheckErrors::PublicFunctionNotReadOnly(contract_identifier, function_name) => format!("function '{}' in '{}' is not read-only", contract_identifier, function_name),
            CheckErrors::ContractAlreadyExists(contract_identifier) => format!("contract name '{}' conflicts with existing contract", contract_identifier),
            CheckErrors::ContractCallExpectName => format!("missing contract name for call"),
            CheckErrors::NoSuchBlockInfoProperty(property_name) => format!("use of block unknown property '{}'", property_name),
            CheckErrors::GetBlockInfoExpectPropertyName => format!("missing property name for block info introspection"),
            CheckErrors::NameAlreadyUsed(name) => format!("defining '{}' conflicts with previous value", name),
            CheckErrors::NonFunctionApplication => format!("expecting expression of type function"),
            CheckErrors::ExpectedListApplication => format!("expecting expression of type list"),
            CheckErrors::ExpectedSequence(found_type) => format!("expecting expression of type 'list', 'buff', 'string-ascii' or 'string-utf8' - found '{}'", found_type),
            CheckErrors::MaxLengthOverflow => format!("expecting a value <= {}", u32::max_value()),
            CheckErrors::BadLetSyntax => format!("invalid syntax of 'let'"),
            CheckErrors::CircularReference(function_names) => format!("detected interdependent functions ({})", function_names.join(", ")),
            CheckErrors::BadSyntaxBinding => format!("invalid syntax binding"),
            CheckErrors::MaxContextDepthReached => format!("reached depth limit"),
            CheckErrors::UndefinedVariable(var_name) => format!("use of unresolved variable '{}'", var_name),
            CheckErrors::UndefinedFunction(var_name) => format!("use of unresolved function '{}'", var_name),
            CheckErrors::RequiresAtLeastArguments(expected, found) => format!("expecting >= {} argument, got {}", expected, found),
            CheckErrors::IncorrectArgumentCount(expected_count, found_count) => format!("expecting {} arguments, got {}", expected_count, found_count),
            CheckErrors::IfArmsMustMatch(type_1, type_2) => format!("expression common returned by the arms of 'if' must match (got '{}' and '{}')", type_1, type_2),
            CheckErrors::MatchArmsMustMatch(type_1, type_2) => format!("expression common returned by the arms of 'match' must match (got '{}' and '{}')", type_1, type_2),
            CheckErrors::DefaultTypesMustMatch(type_1, type_2) => format!("expression common passed in 'default-to' must match (got '{}' and '{}')", type_1, type_2),
            CheckErrors::TooManyExpressions => format!("reached limit of expressions"),
            CheckErrors::IllegalOrUnknownFunctionApplication(function_name) => format!("use of illegal / unresolved function '{}", function_name),
            CheckErrors::UnknownFunction(function_name) => format!("use of unresolved function '{}'", function_name),
            CheckErrors::TraitBasedContractCallInReadOnly => format!("use of trait based contract calls are not allowed in read-only context"),
            CheckErrors::WriteAttemptedInReadOnly => format!("expecting read-only statements, detected a writing operation"),
            CheckErrors::AtBlockClosureMustBeReadOnly => format!("(at-block ...) closures expect read-only statements, but detected a writing operation"),
            CheckErrors::BadTokenName => format!("expecting an token name as an argument"),
            CheckErrors::DefineFTBadSignature => format!("(define-token ...) expects a token name as an argument"),
            CheckErrors::DefineNFTBadSignature => format!("(define-asset ...) expects an asset name and an asset identifier type signature as arguments"),
            CheckErrors::NoSuchNFT(asset_name) => format!("tried to use asset function with a undefined asset ('{}')", asset_name),
            CheckErrors::NoSuchFT(asset_name) => format!("tried to use token function with a undefined token ('{}')", asset_name),
            CheckErrors::TraitReferenceUnknown(trait_name) => format!("use of undeclared trait <{}>", trait_name),
            CheckErrors::TraitMethodUnknown(trait_name, func_name) => format!("method '{}' unspecified in trait <{}>", func_name, trait_name),
            CheckErrors::ImportTraitBadSignature => format!("(use-trait ...) expects a trait name and a trait identifier"),
            CheckErrors::BadTraitImplementation(trait_name, func_name) => format!("invalid signature for method '{}' regarding trait's specification <{}>", func_name, trait_name),
            CheckErrors::ExpectedTraitIdentifier => format!("expecting expression of type trait identifier"),
            CheckErrors::UnexpectedTraitOrFieldReference => format!("unexpected use of trait reference or field"),
            CheckErrors::DefineTraitBadSignature => format!("invalid trait definition"),
            CheckErrors::TraitReferenceNotAllowed => format!("trait references can not be stored"),
            CheckErrors::ContractOfExpectsTrait => format!("trait reference expected"),
            CheckErrors::InvalidCharactersDetected => format!("invalid characters detected"),
            CheckErrors::InvalidSecp65k1Signature => format!("invalid seckp256k1 signature"),
            CheckErrors::TypeAlreadyAnnotatedFailure | CheckErrors::CheckerImplementationFailure => {
                format!("internal error - please file an issue on github.com/blockstack/blockstack-core")
            },
            CheckErrors::UncheckedIntermediaryResponses => format!("intermediary responses in consecutive statements must be checked"),
            CheckErrors::CostComputationFailed(s) => format!("contract cost computation failed: {}", s),
        }
    }

    fn suggestion(&self) -> Option<String> {
        match &self {
            CheckErrors::BadSyntaxBinding => {
                Some(format!("binding syntax example: ((supply int) (ttl int))"))
            }
            CheckErrors::BadLetSyntax => Some(format!(
                "'let' syntax example: (let ((supply 1000) (ttl 60)) <next-expression>)"
            )),
            CheckErrors::TraitReferenceUnknown(_) => Some(format!(
                "traits should be either defined, with define-trait, or imported, with use-trait."
            )),
            CheckErrors::NoSuchBlockInfoProperty(_) => Some(format!(
                "properties available: time, header-hash, burnchain-header-hash, vrf-seed"
            )),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum MarfError {
    NotOpenedError,
    IOError(io::Error),
    SQLError(rusqlite::Error),
    RequestedIdentifierForExtensionTrie,
    NotFoundError,
    BackptrNotFoundError,
    ExistsError,
    BadSeekValue,
    CorruptionError(String),
    BlockHashMapCorruptionError(Option<Box<MarfError>>),
    ReadOnlyError,
    UnconfirmedError,
    NotDirectoryError,
    PartialWriteError,
    InProgressError,
    WriteNotBegunError,
    CursorError(node::CursorError),
    RestoreMarfBlockError(Box<MarfError>),
    NonMatchingForks([u8; 32], [u8; 32]),
}

impl From<io::Error> for MarfError {
    fn from(err: io::Error) -> Self {
        MarfError::IOError(err)
    }
}

impl From<rusqlite::Error> for MarfError {
    fn from(err: rusqlite::Error) -> Self {
        if let rusqlite::Error::QueryReturnedNoRows = err {
            MarfError::NotFoundError
        } else {
            MarfError::SQLError(err)
        }
    }
}

impl From<DBError> for MarfError {
    fn from(e: DBError) -> MarfError {
        match e {
            DBError::SqliteError(se) => MarfError::SQLError(se),
            DBError::NotFoundError => MarfError::NotFoundError,
            _ => MarfError::CorruptionError(format!("{}", &e)),
        }
    }
}

impl fmt::Display for MarfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MarfError::IOError(ref e) => fmt::Display::fmt(e, f),
            MarfError::SQLError(ref e) => fmt::Display::fmt(e, f),
            MarfError::CorruptionError(ref s) => fmt::Display::fmt(s, f),
            MarfError::CursorError(ref e) => fmt::Display::fmt(e, f),
            MarfError::BlockHashMapCorruptionError(ref opt_e) => {
                f.write_str("Corrupted MARF BlockHashMap")?;
                match opt_e {
                    Some(e) => write!(f, ": {}", e),
                    None => Ok(()),
                }
            }
            MarfError::NotOpenedError => write!(f, "Tried to read data from unopened storage"),
            MarfError::NotFoundError => write!(f, "Object not found"),
            MarfError::BackptrNotFoundError => write!(f, "Object not found from backptrs"),
            MarfError::ExistsError => write!(f, "Object exists"),
            MarfError::BadSeekValue => write!(f, "Bad seek value"),
            MarfError::ReadOnlyError => write!(f, "Storage is in read-only mode"),
            MarfError::UnconfirmedError => write!(f, "Storage is in unconfirmed mode"),
            MarfError::NotDirectoryError => write!(f, "Not a directory"),
            MarfError::PartialWriteError => {
                write!(f, "Data is partially written and not yet recovered")
            }
            MarfError::InProgressError => write!(f, "Write was in progress"),
            MarfError::WriteNotBegunError => write!(f, "Write has not begun"),
            MarfError::RestoreMarfBlockError(_) => write!(
                f,
                "Failed to restore previous open block during block header check"
            ),
            MarfError::NonMatchingForks(_, _) => {
                write!(f, "The supplied blocks are not in the same fork")
            }
            MarfError::RequestedIdentifierForExtensionTrie => {
                write!(f, "BUG: MARF requested the identifier for a RAM trie")
            }
        }
    }
}

impl error::Error for MarfError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            MarfError::IOError(ref e) => Some(e),
            MarfError::SQLError(ref e) => Some(e),
            MarfError::RestoreMarfBlockError(ref e) => Some(e),
            MarfError::BlockHashMapCorruptionError(ref opt_e) => match opt_e {
                Some(ref e) => Some(e),
                None => None,
            },
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseErrors {
    CostOverflow,
    CostBalanceExceeded(ExecutionCost, ExecutionCost),
    MemoryBalanceExceeded(u64, u64),
    TooManyExpressions,
    ExpressionStackDepthTooDeep,
    FailedCapturingInput,
    SeparatorExpected(String),
    SeparatorExpectedAfterColon(String),
    ProgramTooLarge,
    IllegalVariableName(String),
    IllegalContractName(String),
    UnknownQuotedValue(String),
    FailedParsingIntValue(String),
    FailedParsingBuffer(String),
    FailedParsingHexValue(String, String),
    FailedParsingPrincipal(String),
    FailedParsingField(String),
    FailedParsingRemainder(String),
    ClosingParenthesisUnexpected,
    ClosingParenthesisExpected,
    ClosingTupleLiteralUnexpected,
    ClosingTupleLiteralExpected,
    CircularReference(Vec<String>),
    TupleColonExpected(usize),
    TupleCommaExpected(usize),
    TupleItemExpected(usize),
    NameAlreadyUsed(String),
    TraitReferenceNotAllowed,
    ImportTraitBadSignature,
    DefineTraitBadSignature,
    ImplTraitBadSignature,
    TraitReferenceUnknown(String),
    CommaSeparatorUnexpected,
    ColonSeparatorUnexpected,
    InvalidCharactersDetected,
    InvalidEscaping,
    CostComputationFailed(String),
}

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub err: ParseErrors,
    pub pre_expressions: Option<Vec<PreSymbolicExpression>>,
    pub diagnostic: Diagnostic,
}

impl ParseError {
    pub fn new(err: ParseErrors) -> ParseError {
        let diagnostic = Diagnostic::err(&err);
        ParseError {
            err,
            pre_expressions: None,
            diagnostic,
        }
    }

    pub fn has_pre_expression(&self) -> bool {
        self.pre_expressions.is_some()
    }

    pub fn set_pre_expression(&mut self, expr: &PreSymbolicExpression) {
        self.diagnostic.spans = vec![expr.span.clone()];
        self.pre_expressions.replace(vec![expr.clone()]);
    }

    pub fn set_pre_expressions(&mut self, exprs: Vec<PreSymbolicExpression>) {
        self.diagnostic.spans = exprs.iter().map(|e| e.span.clone()).collect();
        self.pre_expressions.replace(exprs.to_vec());
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.err {
            _ => write!(f, "{:?}", self.err),
        }?;

        if let Some(ref e) = self.pre_expressions {
            write!(f, "\nNear:\n{:?}", e)?;
        }

        Ok(())
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.err {
            _ => None,
        }
    }
}

impl From<ParseErrors> for ParseError {
    fn from(err: ParseErrors) -> Self {
        ParseError::new(err)
    }
}

impl From<CostErrors> for ParseError {
    fn from(err: CostErrors) -> Self {
        match err {
            CostErrors::CostOverflow => ParseError::new(ParseErrors::CostOverflow),
            CostErrors::CostBalanceExceeded(a, b) => {
                ParseError::new(ParseErrors::CostBalanceExceeded(a, b))
            }
            CostErrors::MemoryBalanceExceeded(a, b) => {
                ParseError::new(ParseErrors::MemoryBalanceExceeded(a, b))
            }
            CostErrors::CostComputationFailed(s) => {
                ParseError::new(ParseErrors::CostComputationFailed(s))
            }
            CostErrors::CostContractLoadFailure => ParseError::new(
                ParseErrors::CostComputationFailed("Failed to load cost contract".into()),
            ),
        }
    }
}

impl DiagnosableError for ParseErrors {
    fn message(&self) -> String {
        match &self {
            ParseErrors::CostOverflow => format!("Used up cost budget during the parse"),
            ParseErrors::CostBalanceExceeded(bal, used) => format!(
                "Used up cost budget during the parse: {} balance, {} used",
                bal, used
            ),
            ParseErrors::MemoryBalanceExceeded(bal, used) => format!(
                "Used up memory budget during the parse: {} balance, {} used",
                bal, used
            ),
            ParseErrors::TooManyExpressions => format!("Too many expressions"),
            ParseErrors::FailedCapturingInput => format!("Failed to capture value from input"),
            ParseErrors::SeparatorExpected(found) => {
                format!("Expected whitespace or a close parens. Found: '{}'", found)
            }
            ParseErrors::SeparatorExpectedAfterColon(found) => {
                format!("Whitespace expected after colon (:), Found: '{}'", found)
            }
            ParseErrors::ProgramTooLarge => format!("Program too large to parse"),
            ParseErrors::IllegalContractName(contract_name) => {
                format!("Illegal contract name: '{}'", contract_name)
            }
            ParseErrors::IllegalVariableName(var_name) => {
                format!("Illegal variable name: '{}'", var_name)
            }
            ParseErrors::UnknownQuotedValue(value) => format!("Unknown 'quoted value '{}'", value),
            ParseErrors::FailedParsingIntValue(value) => {
                format!("Failed to parse int literal '{}'", value)
            }
            ParseErrors::FailedParsingHexValue(value, x) => {
                format!("Invalid hex-string literal {}: {}", value, x)
            }
            ParseErrors::FailedParsingPrincipal(value) => {
                format!("Invalid principal literal: {}", value)
            }
            ParseErrors::FailedParsingBuffer(value) => format!("Invalid buffer literal: {}", value),
            ParseErrors::FailedParsingField(value) => format!("Invalid field literal: {}", value),
            ParseErrors::FailedParsingRemainder(remainder) => {
                format!("Failed to lex input remainder: '{}'", remainder)
            }
            ParseErrors::ClosingParenthesisUnexpected => {
                format!("Tried to close list which isn't open.")
            }
            ParseErrors::ClosingParenthesisExpected => {
                format!("List expressions (..) left opened.")
            }
            ParseErrors::ClosingTupleLiteralUnexpected => {
                format!("Tried to close tuple literal which isn't open.")
            }
            ParseErrors::ClosingTupleLiteralExpected => {
                format!("Tuple literal {{..}} left opened.")
            }
            ParseErrors::ColonSeparatorUnexpected => format!("Misplaced colon."),
            ParseErrors::CommaSeparatorUnexpected => format!("Misplaced comma."),
            ParseErrors::TupleColonExpected(i) => {
                format!("Tuple literal construction expects a colon at index {}", i)
            }
            ParseErrors::TupleCommaExpected(i) => {
                format!("Tuple literal construction expects a comma at index {}", i)
            }
            ParseErrors::TupleItemExpected(i) => format!(
                "Tuple literal construction expects a key or value at index {}",
                i
            ),
            ParseErrors::CircularReference(function_names) => format!(
                "detected interdependent functions ({})",
                function_names.join(", ")
            ),
            ParseErrors::NameAlreadyUsed(name) => {
                format!("defining '{}' conflicts with previous value", name)
            }
            ParseErrors::ImportTraitBadSignature => {
                format!("(use-trait ...) expects a trait name and a trait identifier")
            }
            ParseErrors::DefineTraitBadSignature => {
                format!("(define-trait ...) expects a trait name and a trait definition")
            }
            ParseErrors::ImplTraitBadSignature => {
                format!("(impl-trait ...) expects a trait identifier")
            }
            ParseErrors::TraitReferenceNotAllowed => format!("trait references can not be stored"),
            ParseErrors::TraitReferenceUnknown(trait_name) => {
                format!("use of undeclared trait <{}>", trait_name)
            }
            ParseErrors::ExpressionStackDepthTooDeep => format!(
                "AST has too deep of an expression nesting. The maximum stack depth is {}",
                MAX_CALL_STACK_DEPTH
            ),
            ParseErrors::InvalidCharactersDetected => format!("invalid characters detected"),
            ParseErrors::InvalidEscaping => format!("invalid escaping detected in string"),
            ParseErrors::CostComputationFailed(s) => format!("Cost computation failed: {}", s),
        }
    }

    fn suggestion(&self) -> Option<String> {
        match &self {
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct IncomparableError<T> {
    pub err: T,
}

impl<T> PartialEq<IncomparableError<T>> for IncomparableError<T> {
    fn eq(&self, _other: &IncomparableError<T>) -> bool {
        return false;
    }
}

/// RuntimeErrors are errors that smart contracts are expected
///   to be able to trigger during execution (e.g., arithmetic errors)
#[derive(Debug, PartialEq)]
pub enum RuntimeErrorType {
    Arithmetic(String),
    ArithmeticOverflow,
    ArithmeticUnderflow,
    SupplyOverflow(u128, u128),
    SupplyUnderflow(u128, u128),
    DivisionByZero,
    // error in parsing common
    ParseError(String),
    // error in parsing the AST
    ASTError(ParseError),
    MaxStackDepthReached,
    MaxContextDepthReached,
    ListDimensionTooHigh,
    BadTypeConstruction,
    ValueTooLarge,
    BadBlockHeight(String),
    TransferNonPositiveAmount,
    NoSuchToken,
    NotImplemented,
    NoSenderInContext,
    NonPositiveTokenSupply,
    JSONParseError(IncomparableError<serde_json::Error>),
    AttemptToFetchInTransientContext,
    BadNameValue(&'static str, String),
    UnknownBlockHeaderHash(BlockHeaderHash),
    BadBlockHash(Vec<u8>),
    UnwrapFailure,
}

#[derive(Debug, PartialEq)]
pub enum ShortReturnType {
    ExpectedValue(Value),
    AssertionFailed(Value),
}

impl fmt::Display for RuntimeErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for RuntimeErrorType {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl Into<Value> for ShortReturnType {
    fn into(self) -> Value {
        match self {
            ShortReturnType::ExpectedValue(v) => v,
            ShortReturnType::AssertionFailed(v) => v,
        }
    }
}

/// Enum for passing data for ClientErrors
#[derive(Debug, Clone, PartialEq)]
pub enum ClientError {
    /// Catch-all
    Message(String),
    /// 404
    NotFound(String),
}

impl error::Error for ClientError {
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::Message(s) => write!(f, "{}", s),
            ClientError::NotFound(s) => write!(f, "HTTP path not matched: {}", s),
        }
    }
}

impl From<ChainstateError> for NetworkError {
    fn from(e: ChainstateError) -> NetworkError {
        match e {
            ChainstateError::InvalidStacksBlock(s) => {
                NetworkError::ChainstateError(format!("Invalid stacks block: {}", s))
            }
            ChainstateError::InvalidStacksMicroblock(msg, hash) => NetworkError::ChainstateError(
                format!("Invalid stacks microblock {:?}: {}", hash, msg),
            ),
            ChainstateError::InvalidStacksTransaction(s, _) => {
                NetworkError::ChainstateError(format!("Invalid stacks transaction: {}", s))
            }
            ChainstateError::PostConditionFailed(s) => {
                NetworkError::ChainstateError(format!("Postcondition failed: {}", s))
            }
            ChainstateError::ClarityError(e) => NetworkError::ClarityError(e),
            ChainstateError::DBError(e) => NetworkError::DBError(e),
            ChainstateError::NetError(e) => e,
            ChainstateError::MARFError(e) => NetworkError::MARFError(e),
            ChainstateError::ReadError(e) => NetworkError::ReadError(e),
            ChainstateError::WriteError(e) => NetworkError::WriteError(e),
            _ => NetworkError::ChainstateError(format!("Stacks chainstate error: {:?}", &e)),
        }
    }
}

impl From<DBError> for NetworkError {
    fn from(e: DBError) -> NetworkError {
        NetworkError::DBError(e)
    }
}

impl From<rusqlite::Error> for NetworkError {
    fn from(e: rusqlite::Error) -> NetworkError {
        NetworkError::DBError(DBError::SqliteError(e))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CostErrors {
    CostComputationFailed(String),
    CostOverflow,
    CostBalanceExceeded(ExecutionCost, ExecutionCost),
    MemoryBalanceExceeded(u64, u64),
    CostContractLoadFailure,
}
