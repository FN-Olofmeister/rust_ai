const wasm = require('./pkg/wasm_host.js');

(async () => {
  const res = await wasm.classify('urgent offer', 'reply ASAP!');
  console.log(res);            // { category: '긴급', confidence: 0.9 }
})().catch(console.error);
