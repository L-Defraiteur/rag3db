namespace Billing {
  export interface Policy {
    limit: number;
    retry(): void;
  }
  export class Box implements Policy {
    limit: number = 1;
    untyped;
    constructor(public label: string) {}
    get size(): number { return this.limit; }
    set size(value: number) { this.limit = value; }
    retry(): void {
      function nested() { return 1; }
      nested();
    }
  }
}
