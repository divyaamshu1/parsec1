document.getElementById('git-status').addEventListener('click', async ()=>{
  const res = await fetch('/api/exec', { method: 'POST', body: 'git status' });
  const text = await res.text();
  document.getElementById('output').textContent = text;
});

document.getElementById('run-cmd').addEventListener('click', async ()=>{
  const res = await fetch('/api/exec', { method: 'POST', body: 'echo hello from host' });
  const text = await res.text();
  document.getElementById('output').textContent = text;
});

document.getElementById('reload').addEventListener('click', ()=> location.reload());
