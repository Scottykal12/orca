document.addEventListener('DOMContentLoaded', () => {
    // State variables
    let fullData = [];
    let currentPage = 1;
    let rowsPerPage = 10;

    // Form elements
    const getTableForm = document.getElementById('get-table-form');
    const insertTableForm = document.getElementById('insert-table-form');
    const useTlsCheckbox = document.getElementById('use-tls');

    // Response and table containers
    const getResponseContainer = document.getElementById('get-response-container');
    const tableContainer = document.getElementById('table-container');
    const getErrorEl = document.getElementById('get-error');

    // Insert form response elements
    const insertResponseContainer = document.getElementById('insert-response-container');
    const insertResponseEl = document.getElementById('insert-response');
    const insertErrorEl = document.getElementById('insert-error');

    // Pagination elements
    const paginationContainer = document.getElementById('pagination-container');
    const prevPageBtn = document.getElementById('prev-page-btn');
    const nextPageBtn = document.getElementById('next-page-btn');
    const pageInfo = document.getElementById('page-info');
    const rowsPerPageSelect = document.getElementById('rows-per-page-select');

    // Tab elements
    const tabContainer = document.querySelector('.tabs');

    // Event Listeners
    getTableForm.addEventListener('submit', handleGetTable);
    insertTableForm.addEventListener('submit', handleInsertData);
    tableContainer.addEventListener('click', handleSaveRow);
    prevPageBtn.addEventListener('click', () => changePage(currentPage - 1));
    nextPageBtn.addEventListener('click', () => changePage(currentPage + 1));
    rowsPerPageSelect.addEventListener('change', (e) => {
        rowsPerPage = parseInt(e.target.value, 10);
        changePage(1);
    });

    if (tabContainer) {
        tabContainer.addEventListener('click', (e) => {
            if (e.target.classList.contains('tab-link')) {
                const tabLinks = tabContainer.querySelectorAll('.tab-link');
                const tabContents = tabContainer.querySelectorAll('.tab-content');
                const tabId = e.target.dataset.tab;

                tabLinks.forEach(link => link.classList.remove('active'));
                tabContents.forEach(content => content.classList.remove('active'));

                e.target.classList.add('active');
                document.getElementById(tabId).classList.add('active');
            }
        });
    }


    async function handleGetTable(event) {
        event.preventDefault();
        const tableName = document.getElementById('get-table-name').value;
        
        getResponseContainer.style.display = 'none';
        tableContainer.innerHTML = '';
        getErrorEl.textContent = '';
        paginationContainer.style.display = 'none';

        try {
            const protocol = useTlsCheckbox.checked ? 'https' : 'http';
            const response = await fetch(`${protocol}://127.0.0.1:8082/db/${tableName}`);

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`HTTP error! status: ${response.status}, message: ${errorText}`);
            }

            fullData = await response.json();
            currentPage = 1;
            displayPage();
            getResponseContainer.style.display = 'block';
            if (fullData.length > 0) {
                paginationContainer.style.display = 'flex';
            }

        } catch (error) {
            console.error('Error:', error);
            getErrorEl.textContent = error.message;
            getResponseContainer.style.display = 'block';
        }
    }

    async function handleInsertData(event) {
        event.preventDefault();
        const tableName = document.getElementById('insert-table-name').value;
        const data = document.getElementById('insert-data').value;

        insertResponseContainer.style.display = 'none';
        insertResponseEl.textContent = '';
        insertErrorEl.textContent = '';

        try {
            const protocol = useTlsCheckbox.checked ? 'https' : 'http';
            const response = await fetch(`${protocol}://127.0.0.1:8082/db/${tableName}`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: data
            });

            const responseText = await response.text();

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}, message: ${responseText}`);
            }

            insertResponseEl.textContent = responseText;
            insertResponseContainer.style.display = 'block';

        } catch (error) {
            console.error('Error:', error);
            insertErrorEl.textContent = error.message;
            insertResponseContainer.style.display = 'block';
        }
    }

    function displayPage() {
        const startIndex = (currentPage - 1) * rowsPerPage;
        const endIndex = startIndex + rowsPerPage;
        const paginatedData = fullData.slice(startIndex, endIndex);

        renderTable(paginatedData);
        updatePaginationInfo();
    }

    function renderTable(data) {
        tableContainer.innerHTML = '';
        if (!data || data.length === 0) {
            tableContainer.innerHTML = '<p>No data found.</p>';
            return;
        }

        const table = document.createElement('table');
        const thead = document.createElement('thead');
        const tbody = document.createElement('tbody');
        const headers = Object.keys(data[0]);

        const headerRow = document.createElement('tr');
        headers.forEach(headerText => {
            const th = document.createElement('th');
            th.textContent = headerText;
            headerRow.appendChild(th);
        });
        const actionHeader = document.createElement('th');
        actionHeader.textContent = 'Action';
        headerRow.appendChild(actionHeader);
        thead.appendChild(headerRow);

        data.forEach(rowData => {
            const row = document.createElement('tr');
            row.dataset.originalData = JSON.stringify(rowData);

            headers.forEach(header => {
                const cell = document.createElement('td');
                cell.textContent = rowData[header];
                cell.setAttribute('contenteditable', 'true');
                row.appendChild(cell);
            });

            const actionCell = document.createElement('td');
            const saveButton = document.createElement('button');
            saveButton.textContent = 'Save';
            saveButton.className = 'save-btn';
            actionCell.appendChild(saveButton);
            row.appendChild(actionCell);

            tbody.appendChild(row);
        });

        table.appendChild(thead);
        table.appendChild(tbody);
        tableContainer.appendChild(table);
    }
    
    function updatePaginationInfo() {
        const totalPages = Math.ceil(fullData.length / rowsPerPage);
        pageInfo.textContent = `Page ${currentPage} of ${totalPages}`;
        prevPageBtn.disabled = currentPage === 1;
        nextPageBtn.disabled = currentPage === totalPages;
    }

    function changePage(page) {
        const totalPages = Math.ceil(fullData.length / rowsPerPage);
        if (page < 1 || page > totalPages) {
            return;
        }
        currentPage = page;
        displayPage();
    }

    async function handleSaveRow(event) {
        if (event.target.classList.contains('save-btn')) {
            const row = event.target.closest('tr');
            const originalData = JSON.parse(row.dataset.originalData);
            const tableName = document.getElementById('get-table-name').value;
            const pkColumn = document.getElementById('pk-column-name').value;

            if (!pkColumn) {
                alert('Please specify the primary key column.');
                return;
            }

            const pkValue = originalData[pkColumn];
            const cells = row.querySelectorAll('td[contenteditable="true"]');
            const headers = Array.from(row.parentElement.previousElementSibling.querySelectorAll('th')).map(th => th.textContent);
            const updatedData = {};
            
            cells.forEach((cell, index) => {
                const header = headers[index];
                updatedData[header] = cell.textContent;
            });

            try {
                const protocol = useTlsCheckbox.checked ? 'https' : 'http';
                const response = await fetch(`${protocol}://127.0.0.1:8082/db/${tableName}?pk_col=${pkColumn}&pk_val=${pkValue}`, {
                    method: 'PUT',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify(updatedData)
                });

                const responseText = await response.text();

                if (!response.ok) {
                    throw new Error(`HTTP error! status: ${response.status}, message: ${responseText}`);
                }

                alert('Row updated successfully!');
                row.dataset.originalData = JSON.stringify(updatedData);
                
                // Update the full dataset as well
                const dataIndex = fullData.findIndex(item => item[pkColumn] == pkValue);
                if(dataIndex > -1) {
                    fullData[dataIndex] = updatedData;
                }


            } catch (error) {
                console.error('Error:', error);
                alert(`Failed to update row: ${error.message}`);
            }
        }
    }
});