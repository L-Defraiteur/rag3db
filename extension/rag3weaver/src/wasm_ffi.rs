//! WASM emscripten FFI layer (feature: `wasm-emscripten`).
//!
//! Provides [`WasmDbConnection`] which implements [`DbConnection`] by calling
//! the rag3db C API directly. Symbols resolve at link time when the static
//! library is linked into the emscripten WASM binary alongside rag3db.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};

use async_trait::async_trait;

use crate::connection::{CypherValue, DbConnection, DbError, QueryParam, QueryResult};

// ── C constants ─────────────────────────────────────────────────────────

type CState = u32;
const C_SUCCESS: CState = 0;

#[allow(dead_code)]
mod type_id {
    pub const NODE: u32 = 10;
    pub const REL: u32 = 11;
    pub const SERIAL: u32 = 13;
    pub const BOOL: u32 = 22;
    pub const INT64: u32 = 23;
    pub const INT32: u32 = 24;
    pub const INT16: u32 = 25;
    pub const INT8: u32 = 26;
    pub const UINT64: u32 = 27;
    pub const UINT32: u32 = 28;
    pub const UINT16: u32 = 29;
    pub const UINT8: u32 = 30;
    pub const INT128: u32 = 31;
    pub const DOUBLE: u32 = 32;
    pub const FLOAT: u32 = 33;
    pub const STRING: u32 = 50;
    pub const BLOB: u32 = 51;
    pub const LIST: u32 = 52;
    pub const ARRAY: u32 = 53;
    pub const STRUCT: u32 = 54;
    pub const MAP: u32 = 55;
}

// ── C repr(C) types (matching rag3db.h) ─────────────────────────────────

#[repr(C)]
struct CDatabase {
    _database: *mut c_void,
}

#[repr(C)]
struct CConnection {
    _connection: *mut c_void,
}

#[repr(C)]
struct CPreparedStatement {
    _prepared_statement: *mut c_void,
    _bound_values: *mut c_void,
}

#[repr(C)]
struct CQueryResult {
    _query_result: *mut c_void,
    _is_owned_by_cpp: bool,
}

#[repr(C)]
struct CFlatTuple {
    _flat_tuple: *mut c_void,
    _is_owned_by_cpp: bool,
}

#[repr(C)]
struct CLogicalType {
    _data_type: *mut c_void,
}

#[repr(C)]
struct CValue {
    _value: *mut c_void,
    _is_owned_by_cpp: bool,
}

#[repr(C)]
struct CSystemConfig {
    buffer_pool_size: u64,
    max_num_threads: u64,
    enable_compression: bool,
    read_only: bool,
    max_db_size: u64,
    auto_checkpoint: bool,
    checkpoint_threshold: u64,
}

#[repr(C)]
struct CInternalId {
    table_id: u64,
    offset: u64,
}

// ── extern "C" declarations (subset of rag3db.h) ────────────────────────

extern "C" {
    // System config
    fn rag3db_default_system_config() -> CSystemConfig;

    // Database
    fn rag3db_database_init(
        path: *const c_char,
        config: CSystemConfig,
        out: *mut CDatabase,
    ) -> CState;
    fn rag3db_database_destroy(db: *mut CDatabase);

    // Connection
    fn rag3db_connection_init(db: *mut CDatabase, out: *mut CConnection) -> CState;
    fn rag3db_connection_destroy(conn: *mut CConnection);
    fn rag3db_connection_query(
        conn: *mut CConnection,
        query: *const c_char,
        out: *mut CQueryResult,
    ) -> CState;

    // Prepared statements
    fn rag3db_connection_prepare(
        conn: *mut CConnection,
        query: *const c_char,
        out: *mut CPreparedStatement,
    ) -> CState;
    fn rag3db_connection_execute(
        conn: *mut CConnection,
        stmt: *mut CPreparedStatement,
        out: *mut CQueryResult,
    ) -> CState;
    fn rag3db_prepared_statement_destroy(stmt: *mut CPreparedStatement);
    fn rag3db_prepared_statement_is_success(stmt: *mut CPreparedStatement) -> bool;
    fn rag3db_prepared_statement_get_error_message(stmt: *mut CPreparedStatement) -> *mut c_char;
    fn rag3db_prepared_statement_bind_bool(
        stmt: *mut CPreparedStatement,
        name: *const c_char,
        value: bool,
    ) -> CState;
    fn rag3db_prepared_statement_bind_int64(
        stmt: *mut CPreparedStatement,
        name: *const c_char,
        value: i64,
    ) -> CState;
    fn rag3db_prepared_statement_bind_double(
        stmt: *mut CPreparedStatement,
        name: *const c_char,
        value: f64,
    ) -> CState;
    fn rag3db_prepared_statement_bind_string(
        stmt: *mut CPreparedStatement,
        name: *const c_char,
        value: *const c_char,
    ) -> CState;
    fn rag3db_prepared_statement_bind_value(
        stmt: *mut CPreparedStatement,
        name: *const c_char,
        value: *mut CValue,
    ) -> CState;

    // Query result
    fn rag3db_query_result_destroy(result: *mut CQueryResult);
    fn rag3db_query_result_is_success(result: *mut CQueryResult) -> bool;
    fn rag3db_query_result_get_error_message(result: *mut CQueryResult) -> *mut c_char;
    fn rag3db_query_result_get_num_columns(result: *mut CQueryResult) -> u64;
    fn rag3db_query_result_get_column_name(
        result: *mut CQueryResult,
        index: u64,
        out: *mut *mut c_char,
    ) -> CState;
    fn rag3db_query_result_has_next(result: *mut CQueryResult) -> bool;
    fn rag3db_query_result_get_next(
        result: *mut CQueryResult,
        out: *mut CFlatTuple,
    ) -> CState;

    // Flat tuple
    fn rag3db_flat_tuple_get_value(
        tuple: *mut CFlatTuple,
        index: u64,
        out: *mut CValue,
    ) -> CState;

    // Value: type inspection
    fn rag3db_value_is_null(value: *mut CValue) -> bool;
    fn rag3db_value_get_data_type(value: *mut CValue, out: *mut CLogicalType);
    fn rag3db_data_type_get_id(dt: *mut CLogicalType) -> u32;
    fn rag3db_data_type_destroy(dt: *mut CLogicalType);

    // Value: getters
    fn rag3db_value_get_bool(value: *mut CValue, out: *mut bool) -> CState;
    fn rag3db_value_get_int8(value: *mut CValue, out: *mut i8) -> CState;
    fn rag3db_value_get_int16(value: *mut CValue, out: *mut i16) -> CState;
    fn rag3db_value_get_int32(value: *mut CValue, out: *mut i32) -> CState;
    fn rag3db_value_get_int64(value: *mut CValue, out: *mut i64) -> CState;
    fn rag3db_value_get_uint8(value: *mut CValue, out: *mut u8) -> CState;
    fn rag3db_value_get_uint16(value: *mut CValue, out: *mut u16) -> CState;
    fn rag3db_value_get_uint32(value: *mut CValue, out: *mut u32) -> CState;
    fn rag3db_value_get_uint64(value: *mut CValue, out: *mut u64) -> CState;
    fn rag3db_value_get_float(value: *mut CValue, out: *mut f32) -> CState;
    fn rag3db_value_get_double(value: *mut CValue, out: *mut f64) -> CState;
    fn rag3db_value_get_string(value: *mut CValue, out: *mut *mut c_char) -> CState;
    fn rag3db_value_get_internal_id(value: *mut CValue, out: *mut CInternalId) -> CState;
    fn rag3db_value_to_string(value: *mut CValue) -> *mut c_char;
    fn rag3db_value_create_null() -> *mut CValue;
    fn rag3db_value_destroy(value: *mut CValue);

    // Value: list
    fn rag3db_value_get_list_size(value: *mut CValue, out: *mut u64) -> CState;
    fn rag3db_value_get_list_element(
        value: *mut CValue,
        index: u64,
        out: *mut CValue,
    ) -> CState;

    // Value: struct
    fn rag3db_value_get_struct_num_fields(value: *mut CValue, out: *mut u64) -> CState;
    fn rag3db_value_get_struct_field_name(
        value: *mut CValue,
        index: u64,
        out: *mut *mut c_char,
    ) -> CState;
    fn rag3db_value_get_struct_field_value(
        value: *mut CValue,
        index: u64,
        out: *mut CValue,
    ) -> CState;

    // Value: node
    fn rag3db_node_val_get_label_val(node: *mut CValue, out: *mut CValue) -> CState;
    fn rag3db_node_val_get_id_val(node: *mut CValue, out: *mut CValue) -> CState;
    fn rag3db_node_val_get_property_size(node: *mut CValue, out: *mut u64) -> CState;
    fn rag3db_node_val_get_property_name_at(
        node: *mut CValue,
        index: u64,
        out: *mut *mut c_char,
    ) -> CState;
    fn rag3db_node_val_get_property_value_at(
        node: *mut CValue,
        index: u64,
        out: *mut CValue,
    ) -> CState;

    // Value: rel
    fn rag3db_rel_val_get_label_val(rel: *mut CValue, out: *mut CValue) -> CState;
    fn rag3db_rel_val_get_src_id_val(rel: *mut CValue, out: *mut CValue) -> CState;
    fn rag3db_rel_val_get_dst_id_val(rel: *mut CValue, out: *mut CValue) -> CState;
    fn rag3db_rel_val_get_property_size(rel: *mut CValue, out: *mut u64) -> CState;
    fn rag3db_rel_val_get_property_name_at(
        rel: *mut CValue,
        index: u64,
        out: *mut *mut c_char,
    ) -> CState;
    fn rag3db_rel_val_get_property_value_at(
        rel: *mut CValue,
        index: u64,
        out: *mut CValue,
    ) -> CState;

    // Value: map
    fn rag3db_value_get_map_size(value: *mut CValue, out: *mut u64) -> CState;
    fn rag3db_value_get_map_key(value: *mut CValue, index: u64, out: *mut CValue) -> CState;
    fn rag3db_value_get_map_value(value: *mut CValue, index: u64, out: *mut CValue) -> CState;

    // String cleanup
    fn rag3db_destroy_string(s: *mut c_char);
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Read a C string returned by the rag3db API and free it.
unsafe fn read_api_string(s: *mut c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    let result = CStr::from_ptr(s).to_string_lossy().into_owned();
    rag3db_destroy_string(s);
    result
}

/// Zero-initialized CValue (for use as out parameter).
fn zeroed_value() -> CValue {
    CValue {
        _value: std::ptr::null_mut(),
        _is_owned_by_cpp: false,
    }
}

/// Convert a rag3db C API value to CypherValue.
///
/// Values obtained from flat_tuple/node/rel/list are owned by their parent
/// and must NOT be destroyed here.
unsafe fn value_to_cypher(value: *mut CValue) -> CypherValue {
    if rag3db_value_is_null(value) {
        return CypherValue::Null;
    }

    let mut dt = CLogicalType {
        _data_type: std::ptr::null_mut(),
    };
    rag3db_value_get_data_type(value, &mut dt);
    let tid = rag3db_data_type_get_id(&mut dt);
    rag3db_data_type_destroy(&mut dt);

    match tid {
        type_id::BOOL => {
            let mut v = false;
            rag3db_value_get_bool(value, &mut v);
            CypherValue::Bool(v)
        }
        type_id::INT8 => {
            let mut v: i8 = 0;
            rag3db_value_get_int8(value, &mut v);
            CypherValue::Int(v as i64)
        }
        type_id::INT16 => {
            let mut v: i16 = 0;
            rag3db_value_get_int16(value, &mut v);
            CypherValue::Int(v as i64)
        }
        type_id::INT32 => {
            let mut v: i32 = 0;
            rag3db_value_get_int32(value, &mut v);
            CypherValue::Int(v as i64)
        }
        type_id::INT64 | type_id::SERIAL => {
            let mut v: i64 = 0;
            rag3db_value_get_int64(value, &mut v);
            CypherValue::Int(v)
        }
        type_id::UINT8 => {
            let mut v: u8 = 0;
            rag3db_value_get_uint8(value, &mut v);
            CypherValue::Int(v as i64)
        }
        type_id::UINT16 => {
            let mut v: u16 = 0;
            rag3db_value_get_uint16(value, &mut v);
            CypherValue::Int(v as i64)
        }
        type_id::UINT32 => {
            let mut v: u32 = 0;
            rag3db_value_get_uint32(value, &mut v);
            CypherValue::Int(v as i64)
        }
        type_id::UINT64 => {
            let mut v: u64 = 0;
            rag3db_value_get_uint64(value, &mut v);
            CypherValue::Int(v as i64)
        }
        type_id::INT128 => {
            // Lossy: convert via string representation
            CypherValue::String(read_api_string(rag3db_value_to_string(value)))
        }
        type_id::FLOAT => {
            let mut v: f32 = 0.0;
            rag3db_value_get_float(value, &mut v);
            CypherValue::Float(v as f64)
        }
        type_id::DOUBLE => {
            let mut v: f64 = 0.0;
            rag3db_value_get_double(value, &mut v);
            CypherValue::Float(v)
        }
        type_id::STRING => {
            let mut s: *mut c_char = std::ptr::null_mut();
            rag3db_value_get_string(value, &mut s);
            CypherValue::String(read_api_string(s))
        }
        type_id::LIST | type_id::ARRAY => convert_list(value),
        type_id::NODE => convert_node(value),
        type_id::REL => convert_rel(value),
        type_id::STRUCT => convert_struct(value),
        type_id::MAP => convert_map(value),
        // Fallback: Date, Timestamp, Interval, Blob, UUID, etc.
        _ => CypherValue::String(read_api_string(rag3db_value_to_string(value))),
    }
}

unsafe fn convert_list(value: *mut CValue) -> CypherValue {
    let mut size: u64 = 0;
    rag3db_value_get_list_size(value, &mut size);
    let mut items = Vec::with_capacity(size as usize);
    for i in 0..size {
        let mut elem = zeroed_value();
        rag3db_value_get_list_element(value, i, &mut elem);
        items.push(value_to_cypher(&mut elem));
    }
    CypherValue::List(items)
}

unsafe fn convert_node(value: *mut CValue) -> CypherValue {
    let mut map = HashMap::new();

    // Label
    let mut label_val = zeroed_value();
    rag3db_node_val_get_label_val(value, &mut label_val);
    let mut label_str: *mut c_char = std::ptr::null_mut();
    rag3db_value_get_string(&mut label_val, &mut label_str);
    map.insert("_label".to_string(), CypherValue::String(read_api_string(label_str)));

    // Internal ID
    let mut id_val = zeroed_value();
    rag3db_node_val_get_id_val(value, &mut id_val);
    let mut id = CInternalId { table_id: 0, offset: 0 };
    rag3db_value_get_internal_id(&mut id_val, &mut id);
    map.insert("_id".to_string(), CypherValue::String(format!("{}:{}", id.table_id, id.offset)));

    // Properties
    let mut num_props: u64 = 0;
    rag3db_node_val_get_property_size(value, &mut num_props);
    for i in 0..num_props {
        let mut name_ptr: *mut c_char = std::ptr::null_mut();
        rag3db_node_val_get_property_name_at(value, i, &mut name_ptr);
        let name = read_api_string(name_ptr);

        let mut prop_val = zeroed_value();
        rag3db_node_val_get_property_value_at(value, i, &mut prop_val);
        map.insert(name, value_to_cypher(&mut prop_val));
    }

    CypherValue::Map(map)
}

unsafe fn convert_rel(value: *mut CValue) -> CypherValue {
    let mut map = HashMap::new();

    // Label
    let mut label_val = zeroed_value();
    rag3db_rel_val_get_label_val(value, &mut label_val);
    let mut label_str: *mut c_char = std::ptr::null_mut();
    rag3db_value_get_string(&mut label_val, &mut label_str);
    map.insert("_label".to_string(), CypherValue::String(read_api_string(label_str)));

    // Source ID
    let mut src_val = zeroed_value();
    rag3db_rel_val_get_src_id_val(value, &mut src_val);
    let mut src_id = CInternalId { table_id: 0, offset: 0 };
    rag3db_value_get_internal_id(&mut src_val, &mut src_id);
    map.insert("_src".to_string(), CypherValue::String(format!("{}:{}", src_id.table_id, src_id.offset)));

    // Destination ID
    let mut dst_val = zeroed_value();
    rag3db_rel_val_get_dst_id_val(value, &mut dst_val);
    let mut dst_id = CInternalId { table_id: 0, offset: 0 };
    rag3db_value_get_internal_id(&mut dst_val, &mut dst_id);
    map.insert("_dst".to_string(), CypherValue::String(format!("{}:{}", dst_id.table_id, dst_id.offset)));

    // Properties
    let mut num_props: u64 = 0;
    rag3db_rel_val_get_property_size(value, &mut num_props);
    for i in 0..num_props {
        let mut name_ptr: *mut c_char = std::ptr::null_mut();
        rag3db_rel_val_get_property_name_at(value, i, &mut name_ptr);
        let name = read_api_string(name_ptr);

        let mut prop_val = zeroed_value();
        rag3db_rel_val_get_property_value_at(value, i, &mut prop_val);
        map.insert(name, value_to_cypher(&mut prop_val));
    }

    CypherValue::Map(map)
}

unsafe fn convert_struct(value: *mut CValue) -> CypherValue {
    let mut num_fields: u64 = 0;
    rag3db_value_get_struct_num_fields(value, &mut num_fields);
    let mut map = HashMap::new();
    for i in 0..num_fields {
        let mut name_ptr: *mut c_char = std::ptr::null_mut();
        rag3db_value_get_struct_field_name(value, i, &mut name_ptr);
        let name = read_api_string(name_ptr);

        let mut field_val = zeroed_value();
        rag3db_value_get_struct_field_value(value, i, &mut field_val);
        map.insert(name, value_to_cypher(&mut field_val));
    }
    CypherValue::Map(map)
}

unsafe fn convert_map(value: *mut CValue) -> CypherValue {
    let mut size: u64 = 0;
    rag3db_value_get_map_size(value, &mut size);
    let mut map = HashMap::new();
    for i in 0..size {
        let mut key_val = zeroed_value();
        rag3db_value_get_map_key(value, i, &mut key_val);
        let key = match value_to_cypher(&mut key_val) {
            CypherValue::String(s) => s,
            other => format!("{other:?}"),
        };

        let mut val_val = zeroed_value();
        rag3db_value_get_map_value(value, i, &mut val_val);
        map.insert(key, value_to_cypher(&mut val_val));
    }
    CypherValue::Map(map)
}

// ── WasmDbConnection ────────────────────────────────────────────────────

/// Database connection for WASM emscripten, using the rag3db C API directly.
///
/// The connection owns both the database and connection handles and destroys
/// them on drop. Equivalent to [`crate::rag3db_connection::Rag3dbConnection`]
/// but using raw C FFI instead of the Rust crate.
pub struct WasmDbConnection {
    db: CDatabase,
    conn: std::sync::Mutex<CConnection>,
}

// SAFETY: CDatabase is only accessed during init/drop (same thread).
// CConnection is protected by a Mutex for thread-safe access.
unsafe impl Send for WasmDbConnection {}
unsafe impl Sync for WasmDbConnection {}

impl WasmDbConnection {
    /// Open (or create) a database at the given path.
    /// Pass an empty string for in-memory.
    pub fn new(path: &str) -> Result<Self, DbError> {
        let c_path =
            CString::new(path).map_err(|e| DbError::ConnectionError(e.to_string()))?;

        unsafe {
            let config = rag3db_default_system_config();

            let mut db = CDatabase {
                _database: std::ptr::null_mut(),
            };
            let state = rag3db_database_init(c_path.as_ptr(), config, &mut db);
            if state != C_SUCCESS {
                return Err(DbError::ConnectionError(format!(
                    "rag3db_database_init failed (path={path})"
                )));
            }

            let mut conn = CConnection {
                _connection: std::ptr::null_mut(),
            };
            let state = rag3db_connection_init(&mut db, &mut conn);
            if state != C_SUCCESS {
                rag3db_database_destroy(&mut db);
                return Err(DbError::ConnectionError(
                    "rag3db_connection_init failed".into(),
                ));
            }

            Ok(Self {
                db,
                conn: std::sync::Mutex::new(conn),
            })
        }
    }

    /// Create an in-memory database.
    pub fn in_memory() -> Result<Self, DbError> {
        Self::new("")
    }

    /// Execute a Cypher query (sync).
    fn query_sync(&self, cypher: &str) -> Result<QueryResult, DbError> {
        let c_cypher =
            CString::new(cypher).map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut conn_guard = self
            .conn
            .lock()
            .map_err(|e| DbError::QueryError(format!("conn lock poisoned: {e}")))?;

        unsafe {
            let mut result = CQueryResult {
                _query_result: std::ptr::null_mut(),
                _is_owned_by_cpp: false,
            };

            let conn_ptr = &mut *conn_guard as *mut CConnection;
            let state = rag3db_connection_query(conn_ptr, c_cypher.as_ptr(), &mut result);
            if state != C_SUCCESS {
                return Err(DbError::QueryError(format!(
                    "rag3db_connection_query failed: {cypher}"
                )));
            }

            if !rag3db_query_result_is_success(&mut result) {
                let msg = read_api_string(rag3db_query_result_get_error_message(&mut result));
                rag3db_query_result_destroy(&mut result);
                return Err(DbError::QueryError(msg));
            }

            let qr = self.read_query_result(&mut result);
            rag3db_query_result_destroy(&mut result);
            Ok(qr)
        }
    }

    /// Execute a parameterized Cypher query (sync).
    fn query_with_params_sync(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        let c_cypher =
            CString::new(cypher).map_err(|e| DbError::QueryError(e.to_string()))?;

        let mut conn_guard = self
            .conn
            .lock()
            .map_err(|e| DbError::QueryError(format!("conn lock poisoned: {e}")))?;

        unsafe {
            let conn_ptr = &mut *conn_guard as *mut CConnection;

            // Prepare
            let mut stmt = CPreparedStatement {
                _prepared_statement: std::ptr::null_mut(),
                _bound_values: std::ptr::null_mut(),
            };
            let state = rag3db_connection_prepare(conn_ptr, c_cypher.as_ptr(), &mut stmt);
            if state != C_SUCCESS {
                return Err(DbError::QueryError(format!(
                    "rag3db_connection_prepare failed: {cypher}"
                )));
            }
            if !rag3db_prepared_statement_is_success(&mut stmt) {
                let msg = read_api_string(rag3db_prepared_statement_get_error_message(&mut stmt));
                rag3db_prepared_statement_destroy(&mut stmt);
                return Err(DbError::QueryError(msg));
            }

            // Bind parameters
            for param in params {
                let c_name = CString::new(param.name.as_str())
                    .map_err(|e| DbError::QueryError(e.to_string()))?;
                self.bind_param(&mut stmt, &c_name, &param.value)?;
            }

            // Execute
            let mut result = CQueryResult {
                _query_result: std::ptr::null_mut(),
                _is_owned_by_cpp: false,
            };
            let state = rag3db_connection_execute(conn_ptr, &mut stmt, &mut result);
            rag3db_prepared_statement_destroy(&mut stmt);

            if state != C_SUCCESS {
                return Err(DbError::QueryError(format!(
                    "rag3db_connection_execute failed: {cypher}"
                )));
            }
            if !rag3db_query_result_is_success(&mut result) {
                let msg = read_api_string(rag3db_query_result_get_error_message(&mut result));
                rag3db_query_result_destroy(&mut result);
                return Err(DbError::QueryError(msg));
            }

            let qr = self.read_query_result(&mut result);
            rag3db_query_result_destroy(&mut result);
            Ok(qr)
        }
    }

    /// Bind a CypherValue to a prepared statement parameter.
    unsafe fn bind_param(
        &self,
        stmt: &mut CPreparedStatement,
        c_name: &CString,
        value: &CypherValue,
    ) -> Result<(), DbError> {
        let name_ptr = c_name.as_ptr();
        match value {
            CypherValue::Null => {
                let null_val = rag3db_value_create_null();
                rag3db_prepared_statement_bind_value(stmt, name_ptr, null_val);
                rag3db_value_destroy(null_val);
            }
            CypherValue::Bool(b) => {
                rag3db_prepared_statement_bind_bool(stmt, name_ptr, *b);
            }
            CypherValue::Int(i) => {
                rag3db_prepared_statement_bind_int64(stmt, name_ptr, *i);
            }
            CypherValue::Float(f) => {
                rag3db_prepared_statement_bind_double(stmt, name_ptr, *f);
            }
            CypherValue::String(s) => {
                let c_val = CString::new(s.as_str())
                    .map_err(|e| DbError::QueryError(e.to_string()))?;
                rag3db_prepared_statement_bind_string(stmt, name_ptr, c_val.as_ptr());
            }
            CypherValue::List(_) | CypherValue::Map(_) => {
                // Serialize as JSON string for complex types
                let json = serde_json::to_string(value)
                    .map_err(|e| DbError::TypeError(e.to_string()))?;
                let c_val = CString::new(json)
                    .map_err(|e| DbError::QueryError(e.to_string()))?;
                rag3db_prepared_statement_bind_string(stmt, name_ptr, c_val.as_ptr());
            }
        }
        Ok(())
    }

    /// Read all columns and rows from a C query result.
    unsafe fn read_query_result(&self, result: &mut CQueryResult) -> QueryResult {
        let num_cols = rag3db_query_result_get_num_columns(result);

        // Column names
        let mut columns = Vec::with_capacity(num_cols as usize);
        for i in 0..num_cols {
            let mut name_ptr: *mut c_char = std::ptr::null_mut();
            rag3db_query_result_get_column_name(result, i, &mut name_ptr);
            columns.push(read_api_string(name_ptr));
        }

        // Rows
        let mut rows = Vec::new();
        let mut tuple = CFlatTuple {
            _flat_tuple: std::ptr::null_mut(),
            _is_owned_by_cpp: false,
        };
        while rag3db_query_result_has_next(result) {
            if rag3db_query_result_get_next(result, &mut tuple) != C_SUCCESS {
                break;
            }
            let mut row = Vec::with_capacity(num_cols as usize);
            for col in 0..num_cols {
                let mut val = zeroed_value();
                rag3db_flat_tuple_get_value(&mut tuple, col, &mut val);
                row.push(value_to_cypher(&mut val));
            }
            rows.push(row);
        }

        QueryResult { columns, rows }
    }
}

#[async_trait]
impl DbConnection for WasmDbConnection {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.query_sync(cypher)
    }

    async fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.query_with_params_sync(cypher, params)
    }
}

impl Drop for WasmDbConnection {
    fn drop(&mut self) {
        unsafe {
            let conn = self.conn.get_mut().unwrap_or_else(|e| e.into_inner());
            rag3db_connection_destroy(conn);
            rag3db_database_destroy(&mut self.db);
        }
    }
}

// ── FFI entry points (called from C++ embind wrapper) ───────────────────

use crate::catalog::Catalog;
use crate::config::CatalogConfig;
use crate::embedder::MockEmbedder;

/// Opaque context holding a Catalog (behind Arc<Mutex> for thread-safe sharing),
/// a dedicated rayon thread pool, and a drain serialization lock.
pub struct WeaverContext {
    catalog: std::sync::Arc<std::sync::Mutex<Catalog>>,
    drain_lock: std::sync::Arc<std::sync::Mutex<()>>,
    pool: rayon::ThreadPool,
}

/// Thread-local buffer for returning strings to C.
/// Each call overwrites the previous buffer; the old CString is freed.
/// The returned pointer is valid until the next call on the same thread.
/// Safe because the C++ side copies immediately via `std::string(result)`.
thread_local! {
    static RETURN_BUF: std::cell::RefCell<CString> =
        std::cell::RefCell::new(CString::new("").unwrap());
}

fn return_string_to_c(s: String) -> *const c_char {
    let c = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    RETURN_BUF.with(|buf| {
        *buf.borrow_mut() = c;
        buf.borrow().as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn rag3weaver_version() -> *const c_char {
    // Static string, no allocation needed
    b"0.1.0\0".as_ptr() as *const c_char
}

/// Create a new WeaverContext from a JSON config string.
/// The context creates its own in-memory database via the C API.
/// Returns null on error.
#[no_mangle]
pub extern "C" fn rag3weaver_catalog_new(
    config_json: *const c_char,
    db_path: *const c_char,
) -> *mut WeaverContext {
    let config_str = if config_json.is_null() {
        return std::ptr::null_mut();
    } else {
        unsafe { CStr::from_ptr(config_json).to_string_lossy().into_owned() }
    };

    let path = if db_path.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(db_path).to_string_lossy().into_owned() }
    };

    // Parse config
    let config: CatalogConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    let embedding_dim = config.embedding_dim;

    // Create connection
    let conn = match WasmDbConnection::new(&path) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    // Create mock embedder (zero vectors of configured dim)
    let embedder = MockEmbedder::new(embedding_dim);

    // Create catalog
    let mut catalog = Catalog::new(Box::new(conn), Box::new(embedder), config);

    // Initialize schema (async → block_on)
    if futures::executor::block_on(catalog.initialize()).is_err() {
        return std::ptr::null_mut();
    }

    // Create dedicated rayon pool (4 threads, separate from tantivy's global pool)
    let pool = match rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .thread_name(|i| format!("weaver-pool-{i}"))
        .build()
    {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    Box::into_raw(Box::new(WeaverContext {
        catalog: std::sync::Arc::new(std::sync::Mutex::new(catalog)),
        drain_lock: std::sync::Arc::new(std::sync::Mutex::new(())),
        pool,
    }))
}

/// Destroy a WeaverContext.
#[no_mangle]
pub extern "C" fn rag3weaver_catalog_destroy(ctx: *mut WeaverContext) {
    if !ctx.is_null() {
        unsafe {
            drop(Box::from_raw(ctx));
        }
    }
}

/// Create an entity. Returns a JSON string with the UUID, or error.
/// `fields_json` is a JSON object `{"field": value, ...}`.
#[no_mangle]
pub extern "C" fn rag3weaver_create(
    ctx: *mut WeaverContext,
    entity_type: *const c_char,
    fields_json: *const c_char,
) -> *const c_char {
    if ctx.is_null() || entity_type.is_null() || fields_json.is_null() {
        return return_string_to_c(r#"{"error":"null pointer"}"#.into());
    }

    let ctx = unsafe { &*ctx };
    let entity = unsafe { CStr::from_ptr(entity_type).to_string_lossy() };
    let fields_str = unsafe { CStr::from_ptr(fields_json).to_string_lossy() };

    let fields: HashMap<String, CypherValue> = match serde_json::from_str(&fields_str) {
        Ok(f) => f,
        Err(e) => return return_string_to_c(format!(r#"{{"error":"{}"}}"#, e)),
    };

    let mut catalog = match ctx.catalog.lock() {
        Ok(g) => g,
        Err(e) => {
            return return_string_to_c(format!(r#"{{"error":"catalog lock poisoned: {e}"}}"#))
        }
    };

    match catalog.create(&entity, fields) {
        Ok(entity_ref) => {
            let uuid = entity_ref.uuid().unwrap_or_default();
            return_string_to_c(format!(r#"{{"uuid":"{uuid}"}}"#))
        }
        Err(e) => return_string_to_c(format!(r#"{{"error":"{}"}}"#, e)),
    }
}

/// Drain the operation queue (sync, blocks until complete).
/// Returns a JSON string with flush stats.
#[no_mangle]
pub extern "C" fn rag3weaver_drain(ctx: *mut WeaverContext) -> *const c_char {
    if ctx.is_null() {
        return return_string_to_c(r#"{"error":"null pointer"}"#.into());
    }

    let ctx = unsafe { &*ctx };

    // Serialize drain calls
    let _drain_guard = match ctx.drain_lock.lock() {
        Ok(g) => g,
        Err(e) => {
            return return_string_to_c(format!(
                r#"{{"error":"drain lock poisoned: {e}"}}"#
            ))
        }
    };

    let mut catalog = match ctx.catalog.lock() {
        Ok(g) => g,
        Err(e) => {
            return return_string_to_c(format!(
                r#"{{"error":"catalog lock poisoned: {e}"}}"#
            ))
        }
    };

    let result = futures::executor::block_on(catalog.drain());

    return_string_to_c(format!(
        r#"{{"processed":{},"failed":{},"persisted":{}}}"#,
        result.processed, result.failed, result.persisted,
    ))
}

// ── Async drain (rayon::spawn + callback) ───────────────────────────────

/// Callback type for async operations.
/// Called from a rayon pool thread when the operation completes.
/// `result_json` is valid until the next `return_string_to_c` call on that thread.
/// `user_data` is the opaque value passed by C++ (typically a PendingDrain pointer).
pub type AsyncCallback = extern "C" fn(result_json: *const c_char, user_data: usize);

/// Async drain: spawns drain() on the dedicated rayon pool, calls callback when done.
/// Returns immediately (non-blocking). The callback is invoked from a worker thread.
#[no_mangle]
pub extern "C" fn rag3weaver_drain_async(
    ctx: *const WeaverContext,
    callback: AsyncCallback,
    user_data: usize,
) {
    if ctx.is_null() {
        let msg = CString::new(r#"{"error":"null pointer"}"#).unwrap();
        callback(msg.as_ptr(), user_data);
        return;
    }

    let ctx = unsafe { &*ctx };
    let catalog = ctx.catalog.clone();
    let drain_lock = ctx.drain_lock.clone();

    ctx.pool.spawn(move || {
        let _drain_guard = match drain_lock.lock() {
            Ok(g) => g,
            Err(e) => {
                let json = format!(r#"{{"error":"drain lock poisoned: {e}"}}"#);
                callback(return_string_to_c(json), user_data);
                return;
            }
        };

        let mut cat = match catalog.lock() {
            Ok(g) => g,
            Err(e) => {
                let json = format!(r#"{{"error":"catalog lock poisoned: {e}"}}"#);
                callback(return_string_to_c(json), user_data);
                return;
            }
        };

        let result = futures::executor::block_on(cat.drain());
        drop(cat); // Release catalog lock before callback

        let json = format!(
            r#"{{"processed":{},"failed":{},"persisted":{}}}"#,
            result.processed, result.failed, result.persisted,
        );
        callback(return_string_to_c(json), user_data);
    });
}

// ── Threading validation tests ──────────────────────────────────────────

/// Test A: std::thread::spawn + atomics.
/// Spawns N threads, each incrementing a shared atomic counter.
/// Returns JSON: {"ok":bool,"total":N,"expected":N,"threads":N}
#[no_mangle]
pub extern "C" fn rag3weaver_test_threads() -> *const c_char {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let num_threads: u64 = 4;
    let iterations: u64 = 1000;
    let counter = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let c = counter.clone();
            std::thread::spawn(move || {
                for _ in 0..iterations {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    let mut join_errors = 0u64;
    for h in handles {
        if h.join().is_err() {
            join_errors += 1;
        }
    }

    let total = counter.load(Ordering::SeqCst);
    let expected = num_threads * iterations;
    let ok = total == expected && join_errors == 0;

    return_string_to_c(format!(
        r#"{{"ok":{},"total":{},"expected":{},"threads":{},"joinErrors":{}}}"#,
        ok, total, expected, num_threads, join_errors
    ))
}

/// Test B: futures::executor::ThreadPool (async multi-thread without tokio/mio).
/// Spawns async tasks on a thread pool, each incrementing a shared atomic counter.
/// Returns JSON: {"ok":bool,"total":N,"expected":N,"poolSize":N}
#[no_mangle]
pub extern "C" fn rag3weaver_test_async_pool() -> *const c_char {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let result = std::panic::catch_unwind(|| {
        let pool = match futures::executor::ThreadPool::builder()
            .pool_size(2)
            .create()
        {
            Ok(p) => p,
            Err(e) => {
                return format!(r#"{{"ok":false,"error":"pool creation failed: {}"}}"#, e);
            }
        };

        let num_tasks: u64 = 8;
        let counter = Arc::new(AtomicU64::new(0));

        let handles: Vec<_> = (0..num_tasks)
            .map(|_| {
                let c = counter.clone();
                futures::task::SpawnExt::spawn_with_handle(&pool, async move {
                    c.fetch_add(1, Ordering::Relaxed);
                })
                .unwrap()
            })
            .collect();

        // Wait for all tasks to complete
        for h in handles {
            futures::executor::block_on(h);
        }

        let total = counter.load(Ordering::SeqCst);
        let expected = num_tasks;
        let ok = total == expected;

        format!(
            r#"{{"ok":{},"total":{},"expected":{},"poolSize":2}}"#,
            ok, total, expected
        )
    });

    match result {
        Ok(json) => return_string_to_c(json),
        Err(_) => return_string_to_c(
            r#"{"ok":false,"error":"panic during async pool test"}"#.to_string(),
        ),
    }
}

/// Test C: rayon par_iter (data parallelism, same lib as tantivy).
/// Parallel sum using rayon's work-stealing thread pool.
/// Returns JSON: {"ok":bool,"total":N,"expected":N,"numChunks":N}
#[no_mangle]
pub extern "C" fn rag3weaver_test_rayon() -> *const c_char {
    let result = std::panic::catch_unwind(|| {
        use rayon::prelude::*;

        let data: Vec<u64> = (1..=1000).collect();
        let total: u64 = data.par_iter().sum();
        let expected: u64 = (1000 * 1001) / 2; // sum 1..=1000

        // Also test par_chunks to verify real parallelism
        let chunk_results: Vec<u64> = data
            .par_chunks(250)
            .map(|chunk| chunk.iter().sum::<u64>())
            .collect();
        let chunk_total: u64 = chunk_results.iter().sum();
        let num_chunks = chunk_results.len();

        let ok = total == expected && chunk_total == expected;

        format!(
            r#"{{"ok":{},"total":{},"expected":{},"numChunks":{}}}"#,
            ok, total, expected, num_chunks
        )
    });

    match result {
        Ok(json) => return_string_to_c(json),
        Err(_) => return_string_to_c(
            r#"{"ok":false,"error":"panic during rayon test"}"#.to_string(),
        ),
    }
}

/// Test D: tokio current_thread runtime + real thread offload.
/// tokio rt-multi-thread is explicitly blocked on WASM (compile_error! in tokio).
/// Instead we test current_thread runtime + manual std::thread for offload.
/// Returns JSON: {"ok":bool,"total":N,"expected":N,"runtime":"..."}
#[no_mangle]
pub extern "C" fn rag3weaver_test_tokio_mt() -> *const c_char {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let result = std::panic::catch_unwind(|| {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                return format!(r#"{{"ok":false,"error":"runtime build failed: {}"}}"#, e);
            }
        };

        let num_tasks: u64 = 4;
        let counter = Arc::new(AtomicU64::new(0));

        // Use tokio current_thread + oneshot channels for async coordination
        // with real std::threads doing the work
        let total = rt.block_on(async {
            let mut receivers = vec![];
            for _ in 0..num_tasks {
                let c = counter.clone();
                let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                std::thread::spawn(move || {
                    c.fetch_add(1, Ordering::Relaxed);
                    let _ = tx.send(());
                });
                receivers.push(rx);
            }
            for rx in receivers {
                let _ = rx.await;
            }
            counter.load(Ordering::SeqCst)
        });

        let ok = total == num_tasks;

        format!(
            r#"{{"ok":{},"total":{},"expected":{},"runtime":"current_thread+std_threads"}}"#,
            ok, total, num_tasks
        )
    });

    match result {
        Ok(json) => return_string_to_c(json),
        Err(_) => return_string_to_c(
            r#"{"ok":false,"error":"panic during tokio test"}"#.to_string(),
        ),
    }
}

/// Get the entity count for a given entity type. Returns JSON.
#[no_mangle]
pub extern "C" fn rag3weaver_count(
    ctx: *mut WeaverContext,
    entity_type: *const c_char,
) -> *const c_char {
    if ctx.is_null() || entity_type.is_null() {
        return return_string_to_c(r#"{"error":"null pointer"}"#.into());
    }

    let ctx = unsafe { &*ctx };
    let entity = unsafe { CStr::from_ptr(entity_type).to_string_lossy() };

    let catalog = match ctx.catalog.lock() {
        Ok(g) => g,
        Err(e) => {
            return return_string_to_c(format!(
                r#"{{"error":"catalog lock poisoned: {e}"}}"#
            ))
        }
    };

    match futures::executor::block_on(catalog.count(&entity)) {
        Ok(n) => return_string_to_c(format!(r#"{{"count":{n}}}"#)),
        Err(e) => return_string_to_c(format!(r#"{{"error":"{}"}}"#, e)),
    }
}
