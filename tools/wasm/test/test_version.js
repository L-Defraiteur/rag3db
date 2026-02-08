const { assert } = require("chai");

describe("Get version", function () {
  it("should get the version of the library", async function () {
    const version = await rag3db.getVersion();
    assert.isString(version);
    assert.notEqual(version, "");
  });

  it("should get the storage version of the library", async function () {
    const storageVersion = await rag3db.getStorageVersion();
    assert.isTrue(storageVersion > 0);
  });
});
