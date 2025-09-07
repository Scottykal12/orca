
document.addEventListener('DOMContentLoaded', async () => {
    const clientTableContainer = document.getElementById('client-table-container');
    const dispatchForm = document.getElementById('dispatch-form');
    const responseContainer = document.getElementById('response-container');
    const multiResponseContainer = document.getElementById('multi-response-container');
    const useTlsCheckbox = document.getElementById('use-tls');

    async function loadClients() {
        clientTableContainer.innerHTML = '<p>Loading clients...</p>';
        try {
            const protocol = useTlsCheckbox.checked ? 'https' : 'http';
            const response = await fetch(`${protocol}://127.0.0.1:8082/db/clients`);

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const clients = await response.json();
            renderClientTable(clients);

        } catch (error) {
            console.error('Error loading clients:', error);
            clientTableContainer.innerHTML = '<p>Could not load clients. Make sure the API is running and the clients table exists.</p>';
        }
    }

    function renderClientTable(clients) {
        clientTableContainer.innerHTML = '';
        if (!clients || clients.length === 0) {
            clientTableContainer.innerHTML = '<p>No clients found.</p>';
            return;
        }

        const table = document.createElement('table');
        const thead = document.createElement('thead');
        const tbody = document.createElement('tbody');

        const headerRow = document.createElement('tr');
        const selectHeader = document.createElement('th');
        const selectAllCheckbox = document.createElement('input');
        selectAllCheckbox.type = 'checkbox';
        selectAllCheckbox.addEventListener('change', (e) => {
            tbody.querySelectorAll('input[type="checkbox"]').forEach(checkbox => {
                checkbox.checked = e.target.checked;
            });
        });
        selectHeader.appendChild(selectAllCheckbox);
        headerRow.appendChild(selectHeader);

        ['Hostname', 'IP Address'].forEach(headerText => {
            const th = document.createElement('th');
            th.textContent = headerText;
            headerRow.appendChild(th);
        });
        thead.appendChild(headerRow);

        clients.forEach(client => {
            const row = document.createElement('tr');
            const selectCell = document.createElement('td');
            const checkbox = document.createElement('input');
            checkbox.type = 'checkbox';
            checkbox.value = client.ip;
            selectCell.appendChild(checkbox);
            row.appendChild(selectCell);

            const hostnameCell = document.createElement('td');
            hostnameCell.textContent = client.hostname || 'Unknown Host';
            row.appendChild(hostnameCell);

            const ipCell = document.createElement('td');
            ipCell.textContent = client.ip;
            row.appendChild(ipCell);

            tbody.appendChild(row);
        });

        table.appendChild(thead);
        table.appendChild(tbody);
        clientTableContainer.appendChild(table);
    }

    dispatchForm.addEventListener('submit', async function(event) {
        event.preventDefault();

        const selectedCheckboxes = clientTableContainer.querySelectorAll('input[type="checkbox"]:checked');
        const clients = Array.from(selectedCheckboxes).map(cb => cb.value);
        
        if (clients.length === 0) {
            alert('Please select at least one client.');
            return;
        }

        const command = document.getElementById('command').value;
        const files = document.getElementById('files').value;

        responseContainer.style.display = 'block';
        multiResponseContainer.innerHTML = '<h3>Dispatching...</h3>';

        const protocol = useTlsCheckbox.checked ? 'https' : 'http';
        
        const promises = clients.map(client => {
            return fetch(`${protocol}://127.0.0.1:8082/dispatch`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ command, client, files })
            }).then(response => response.json().then(data => ({ client, data })))
              .catch(error => ({ client, error: error.message }));
        });

        const results = await Promise.all(promises);
        
        multiResponseContainer.innerHTML = ''; // Clear "Dispatching..." message
        results.forEach(result => {
            const resultDiv = document.createElement('div');
            resultDiv.className = 'response-item';
            
            let content = `<h4>Client: ${result.client}</h4>`;
            if (result.error) {
                content += `<p><strong>Error:</strong> <span class="stderr">${result.error}</span></p>`;
            } else {
                content += `<p><strong>Success:</strong> <span class="${result.data.success ? 'success-true' : 'success-false'}">${result.data.success}</span></p>`;
                content += `<h5>Stdout:</h5><pre class="stdout">${result.data.stdout || '(empty)'}</pre>`;
                content += `<h5>Stderr:</h5><pre class="stderr">${result.data.stderr || '(empty)'}</pre>`;
            }
            resultDiv.innerHTML = content;
            multiResponseContainer.appendChild(resultDiv);
        });
    });

    useTlsCheckbox.addEventListener('change', loadClients);

    // Initial load
    await loadClients();
});
