// Test: HTTP/HTTPS module for Perry
import https from 'https';

console.log('Starting HTTP tests...');

// Test 1: https.get with URL string
// Note: Using simple non-capture approach to avoid mutable closure capture bug
https.get('https://httpbin.org/get', (res: IncomingMessage) => {
  console.log('GET Status:', res.statusCode);
  console.log('GET StatusMessage:', res.statusMessage);

  res.on('data', (chunk: string) => {
    console.log('GET chunk length:', chunk.length);
    console.log('GET First 100 chars:', chunk.substring(0, 100));
  });

  res.on('end', () => {
    console.log('GET ended');
  });
});

// Test 2: https.request with POST
const options = {
  hostname: 'httpbin.org',
  port: 443,
  path: '/post',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
};

const req = https.request(options, (res: IncomingMessage) => {
  console.log('POST Status:', res.statusCode);

  res.on('data', (chunk: string) => {
    console.log('POST chunk length:', chunk.length);
    // Parse the response body
    const parsed = JSON.parse(chunk);
    console.log('POST echoed data:', JSON.stringify(parsed.data));
  });

  res.on('end', () => {
    console.log('POST ended');
  });
});

req.on('error', (e: any) => {
  console.error('Request error:', e);
});

req.write(JSON.stringify({ hello: 'world' }));
req.end();

// Keep event loop alive so async HTTP callbacks fire
await new Promise<void>((resolve) => setTimeout(resolve, 10000));
console.log('All tests done!');
