/**
 * This file is a customized loader for the rag3dbjs.node native module.
 * It is used to load the native module with the correct flags on Linux so that
 * extension loading works correctly.
 * @module rag3db_native
 * @private
 */

const process = require("process");
const constants = require("constants");
const join = require("path").join;

const rag3dbNativeModule = { exports: {} };
const modulePath = join(__dirname, "rag3dbjs.node");
if (process.platform === "linux") {
  process.dlopen(
    rag3dbNativeModule,
    modulePath,
    constants.RTLD_LAZY | constants.RTLD_GLOBAL
  );
} else {
  process.dlopen(rag3dbNativeModule, modulePath);
}

module.exports = rag3dbNativeModule.exports;
