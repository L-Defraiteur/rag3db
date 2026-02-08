const { assert } = require("chai");

describe("Get version", function () {
  it("should get the version of the library", function () {
    assert.isString(rag3db.VERSION);
    assert.notEqual(rag3db.VERSION, "");
  });

  it("should get the storage version of the library", function () {
    assert.isNumber(rag3db.STORAGE_VERSION);
    assert.isAtLeast(rag3db.STORAGE_VERSION, 1);
  });
});
