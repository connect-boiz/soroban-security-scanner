class HeadersPolyfill {
  constructor(init = {}) {
    this.map = new Map();
    if (init instanceof HeadersPolyfill) {
      init.forEach((value, key) => this.set(key, value));
    } else if (init && typeof init.forEach === 'function') {
      init.forEach((value, key) => this.set(key, value));
    } else {
      Object.entries(init).forEach(([key, value]) => this.set(key, value));
    }
  }

  set(key, value) {
    this.map.set(String(key).toLowerCase(), String(value));
  }

  get(key) {
    return this.map.get(String(key).toLowerCase()) || null;
  }

  has(key) {
    return this.map.has(String(key).toLowerCase());
  }

  delete(key) {
    this.map.delete(String(key).toLowerCase());
  }

  append(key, value) {
    const normalizedKey = String(key).toLowerCase();
    const existing = this.map.get(normalizedKey);
    this.map.set(normalizedKey, existing ? `${existing}, ${value}` : String(value));
  }

  forEach(callback, thisArg) {
    this.map.forEach((value, key) => callback.call(thisArg, value, key, this));
  }

  entries() {
    return this.map.entries();
  }

  keys() {
    return this.map.keys();
  }

  values() {
    return this.map.values();
  }

  [Symbol.iterator]() {
    return this.entries();
  }
}

class RequestPolyfill {
  constructor(input, init = {}) {
    this.url = String(input);
    this.method = init.method || 'GET';
    this.headers = new HeadersPolyfill(init.headers || {});
  }
}

class ResponsePolyfill {
  constructor(body = '', init = {}) {
    this._body = body;
    this.status = init.status || 200;
    this.ok = this.status >= 200 && this.status < 300;
    this.headers = new HeadersPolyfill(init.headers || {});
  }

  async text() {
    return this._body;
  }
}

global.Headers = HeadersPolyfill;
global.Request = RequestPolyfill;
global.Response = ResponsePolyfill;
