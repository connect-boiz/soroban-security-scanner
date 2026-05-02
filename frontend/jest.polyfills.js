class HeadersPolyfill {
  constructor(init = {}) {
    this.map = new Map();
    Object.entries(init).forEach(([key, value]) => this.set(key, value));
  }

  set(key, value) {
    this.map.set(String(key).toLowerCase(), String(value));
  }

  get(key) {
    return this.map.get(String(key).toLowerCase()) || null;
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
