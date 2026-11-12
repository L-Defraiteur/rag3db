class Box {
  count = 1;
  noType;
  constructor(label) { this.label = label; }
  get size() { return this.count; }
  set size(value) { this.count = value; }
  retry() {
    function nested() { return 1; }
    return nested();
  }
}
