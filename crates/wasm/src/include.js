export class WasmResult {
  constructor(value) {
    if (value instanceof Error) {
      this.Err = value;
    } else {
      this.Ok = value;
    }
  }
  is_some() {
    if (this.hasOwnProperty("Err")) {
      return false;
    }
    return true;
  }
  is_none() {
    return !this.is_some();
  }
  unwrap() {
    if (this.hasOwnProperty("Err")) {
      throw this.Err;
    } else {
      return this.Ok
    }
  }
  unwrap_or(otherwise) {
    if (this.hasOwnProperty("Err")) {
      return otherwise;
    } else {
      return this.Ok;
    }
  }
  map(func) {
    if (this.hasOwnProperty("Err")) {
      return this;
    } else {
      return new WasmResult(func(this.Ok));
    }
  }
  map_or(otherwise, func) {
    if (this.hasOwnProperty("Err")) {
      return otherwise;
    } else {
      return func(this.Ok);
    }
  }
}

export class CiteprocRsError extends Error {
    constructor(message) {
        super(message);
        this.name = "CiteprocRsError";
    }
}
export class CiteprocRsDriverError extends CiteprocRsError {
    constructor(message, data) {
        super(message);
        this.data = data;
        this.name = "CiteprocRsDriverError";
    }
}
export class CslStyleError extends CiteprocRsError {
    constructor(message, data) {
        super(message);
        this.data = data;
        this.name = "CslStyleError";
    }
}

// For use in no-modules builds for the browser, which have no linking
// Also because wasm-bindgen is not yet capable of exporting JS items defined here
// to the wasm library consumer.
let env_global;
if (typeof self !== "undefined") {
  env_global = self;
} else if (typeof global !== "undefined") {
  env_global = global;
} else if (typeof window !== "undefined") {
  env_global = window;
}
if (typeof env_global !== "undefined") {
  env_global.WasmResult = WasmResult;
  env_global.CiteprocRsError = CiteprocRsError;
  env_global.CslStyleError = CslStyleError;
  env_global.CiteprocRsDriverError = CiteprocRsDriverError;
}
