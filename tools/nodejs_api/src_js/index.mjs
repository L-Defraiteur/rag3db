import rag3db from "./index.js";

// Re-export everything from the CommonJS module
export const Database = rag3db.Database;
export const Connection = rag3db.Connection;
export const PreparedStatement = rag3db.PreparedStatement;
export const QueryResult = rag3db.QueryResult;
export const VERSION = rag3db.VERSION;
export const STORAGE_VERSION = rag3db.STORAGE_VERSION;
export default rag3db;
