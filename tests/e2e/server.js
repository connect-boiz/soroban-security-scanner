const http = require('http');
const port = process.env.PORT || 3000;

const HTML = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Soroban Security Scanner</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { font-family: system-ui, sans-serif; background: #f8fafc; color: #0f172a; line-height: 1.5; }
    nav { background: #fff; border-bottom: 1px solid #e2e8f0; padding: 1rem 2rem; display: flex; gap: 1.5rem; }
    nav a { color: #475569; text-decoration: none; font-weight: 500; }
    nav a:hover { color: #2563eb; }
    main { max-width: 960px; margin: 2rem auto; padding: 0 1rem; }
    h1 { font-size: 1.875rem; font-weight: 700; margin-bottom: 1.5rem; }
    .form { display: flex; gap: 0.75rem; margin-bottom: 2rem; }
    label { display: block; margin-bottom: 0.5rem; font-weight: 500; }
    input { flex: 1; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 0.5rem; font-size: 1rem; }
    button { padding: 0.75rem 1.5rem; background: #2563eb; color: #fff; border: none; border-radius: 0.5rem; font-size: 1rem; cursor: pointer; }
    button:hover { background: #1d4ed8; }
    .container { max-width: 960px; margin: 0 auto; padding: 0 1rem; }
  </style>
</head>
<body>
  <nav role="navigation">
    <a href="/">Home</a>
    <a href="/results">Results</a>
    <a href="/scan/test">Scan</a>
  </nav>
  <main role="main">
    <div class="container">
      <h1>Soroban Security Scanner</h1>
      <div class="form">
        <label for="contract-input">Contract ID</label>
        <input id="contract-input" type="text" role="textbox" name="contract" placeholder="Enter contract ID..." />
        <button type="button">Scan</button>
      </div>
      <div id="content"></div>
    </div>
  </main>
  <script>
    var path = window.location.pathname;
    if (path === '/results') {
      document.getElementById('content').innerHTML = '<div data-testid="vulnerability-card" class="card"><h2>Vulnerability Report</h2><p>No vulnerabilities found.</p></div>';
    } else if (path.startsWith('/scan/')) {
      document.getElementById('content').innerHTML = '<div class="error"><p>Invalid contract ID. Please check and try again.</p></div>';
    }
  </script>
</body>
</html>`;

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/html' });
  res.end(HTML);
});

server.listen(port, () => {
  console.log(`Test server running at http://localhost:${port}`);
});
