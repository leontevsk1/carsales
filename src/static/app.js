document.addEventListener('DOMContentLoaded', async () => {
    const form = document.getElementById('prediction-form');
    const submitBtn = document.getElementById('submit-btn');
    const resultCard = document.getElementById('result-card');
    
    // Глобальная переменная для хранения словарей, чтобы использовать их при валидации
    // Глобальные переменные для хранения словарей
    let globalMappings = {};
    const memoryOptions = {}; // <-- Здесь будут лежать отсортированные массивы

    // 1. Загрузка словарей из Rust-бэкенда
    try {
        const response = await fetch('/api/mappings');
        if (!response.ok) throw new Error('Не удалось загрузить словари');
        globalMappings = await response.json();

        const inputsWithList = form.querySelectorAll('input[list]');
        
        inputsWithList.forEach(input => {
            const listId = input.getAttribute('list');
            const datalist = document.getElementById(listId);
            const dictKey = input.getAttribute('data-dict') || input.id;
            const dict = globalMappings[dictKey];

            if (dict && datalist) {
                // Сохраняем все варианты в быструю память JS, а не в тяжелый HTML
                memoryOptions[input.id] = Object.keys(dict).sort();
                
                // Функция-рендерер: отдает браузеру только топ-50 результатов
                const renderDatalist = (query) => {
                    const lowerQuery = query.toLowerCase();
                    // JS фильтрует массивы из 5000 элементов за доли миллисекунды
                    const filtered = memoryOptions[input.id]
                        .filter(opt => opt.toLowerCase().includes(lowerQuery))
                        .slice(0, 50); // <-- Жестко ограничиваем объем для рендера
                    
                    datalist.innerHTML = '';
                    filtered.forEach(opt => {
                        const optionElement = document.createElement('option');
                        optionElement.value = opt;
                        datalist.appendChild(optionElement);
                    });
                };

                // При фокусе на поле (даже пустом) показываем первые 50 вариантов
                input.addEventListener('focus', (e) => renderDatalist(e.target.value));
                
                // При вводе текста обновляем подсказки
                input.addEventListener('input', (e) => renderDatalist(e.target.value));
                
            } else {
                input.placeholder = "Словарь не найден";
                input.disabled = true;
            }
        });
    } catch (error) {
        console.error('Ошибка загрузки маппингов:', error);
        alert('Не удалось загрузить списки категорий из модели.');
    }    // 2. Обработка отправки формы
    form.addEventListener('submit', async (e) => {
        e.preventDefault();
        form.classList.add('submitted');

        // Базовая HTML5 валидация (проверка на пустые required поля)
        if (!form.checkValidity()) return;

        // --- НОВАЯ СТРОГАЯ ВАЛИДАЦИЯ СЛОВАРЕЙ ---
        // Проверяем, что текст, введенный пользователем, реально существует в JSON-словарях
        const inputsWithList = form.querySelectorAll('input[list]');
        for (const input of inputsWithList) {
            const dictKey = input.getAttribute('data-dict') || input.id;
            const dict = globalMappings[dictKey];
            const enteredValue = input.value;

            // Если словарь существует, но введенного значения в нем нет
            if (dict && !dict.hasOwnProperty(enteredValue)) {
                alert(`Значение "${enteredValue}" в поле недопустимо. Пожалуйста, выберите существующий вариант из выпадающей подсказки.`);
                input.focus();
                input.style.borderColor = 'var(--error-color)';
                return; // Прерываем отправку формы
            } else {
                input.style.borderColor = '#333'; // Сбрасываем ошибку, если всё ок
            }
        }
        // ----------------------------------------

        submitBtn.disabled = true;
        submitBtn.textContent = 'Обработка...';
        resultCard.classList.add('hidden');

        try {
            const formData = new FormData(form);
            const payload = {};

            for (const [key, value] of formData.entries()) {
                if (value === "") continue; 
                
                const inputElement = form.elements[key];
                if (inputElement.type === 'number') {
                    payload[key] = parseFloat(value);
                } else {
                    payload[key] = value;
                }
            }

            const response = await fetch('/predict', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (!response.ok) throw new Error(`Ошибка сервера: ${response.status}`);

            const data = await response.json();

            document.getElementById('predicted-price').textContent = new Intl.NumberFormat('ru-RU', { 
                style: 'currency', 
                currency: 'RUB',
                maximumFractionDigits: 0 
            }).format(data.predicted_price);
            
            document.getElementById('verdict-text').textContent = data.verdict;
            resultCard.classList.remove('hidden');

        } catch (error) {
            console.error('Ошибка:', error);
            alert('Ошибка при оценке. Проверьте консоль.');
        } finally {
            submitBtn.disabled = false;
            submitBtn.textContent = 'Оценить';
        }
    });
});
