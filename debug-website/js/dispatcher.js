
document.getElementById('dispatch-form').addEventListener('submit', async function(event) {
    event.preventDefault();

    const form = event.target;
    const formData = new FormData(form);
    const command = formData.get('command');
    const client = formData.get('client');
    const files = formData.get('files');

    const responseContainer = document.getElementById('response-container');
    const successEl = document.getElementById('success');
    const stdoutEl = document.getElementById('stdout');
    const stderrEl = document.getElementById('stderr');

    responseContainer.style.display = 'none';

    try {
        // The default port for the api is 8082, as seen in the api.md
        const response = await fetch('http://127.0.0.1:8082/dispatch', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ command, client, files })
        });

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();

        successEl.textContent = data.success;
        successEl.className = data.success ? 'success-true' : 'success-false';
        stdoutEl.textContent = data.stdout || '(empty)';
        stderrEl.textContent = data.stderr || '(empty)';

        responseContainer.style.display = 'block';

    } catch (error) {
        console.error('Error:', error);
        stderrEl.textContent = error.message;
        successEl.textContent = 'false';
        successEl.className = 'success-false';
        responseContainer.style.display = 'block';
    }
});
