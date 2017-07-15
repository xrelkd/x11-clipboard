use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::collections::HashMap;
use ::connection::UnsafeConnection;
use ::{ INCR_CHUNK_SIZE, Connection, SetMap, Atom };


pub fn run(UnsafeConnection(context): UnsafeConnection, setmap: SetMap, max: usize, receiver: &Receiver<Atom>) {
    // TODO
}
