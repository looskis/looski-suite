const token = location.hash.slice(1);
history.replaceState(null, '', '/setup');
const form = document.querySelector('form');
const message = document.querySelector('#message');
const button = document.querySelector('button');
if (!/^[a-f0-9]{64}$/.test(token)) {
  message.textContent = 'Open the complete private setup link provided by your installer.';
  button.disabled = true;
}
form.addEventListener('submit', async (event) => {
  event.preventDefault();
  const data = Object.fromEntries(new FormData(form));
  if (data.password !== data.confirm) {
    message.textContent = 'The passwords do not match.';
    return;
  }
  delete data.confirm;
  button.disabled = true;
  message.textContent = 'Creating your workspace…';
  try {
    const response = await fetch('/setup/owner', {
      method: 'POST', headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
      body: JSON.stringify(data), credentials: 'omit', redirect: 'error',
    });
    if (response.status === 201) {
      form.reset();
      location.replace('/login');
      return;
    }
    message.textContent = response.status === 404
      ? 'This setup link has expired or the owner account already exists. Ask your installer to check.'
      : 'Setup could not finish. Check your details and retry with the same email and password.';
  } catch {
    message.textContent = 'Connection interrupted. Retry with the same email and password.';
  }
  button.disabled = false;
});
