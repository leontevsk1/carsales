document.addEventListener('DOMContentLoaded', async () => {
    const form = document.getElementById('prediction-form');
    const submitBtn = document.getElementById('submit-btn');
    const resultCard = document.getElementById('result-card');
    
    // 1. Загрузка словарей из Rust-бэкенда
    try {
        const response = await fetch('/api/mappings');
        if (!response.ok) throw new Error('Не удалось загрузить словари');
        const mappings = await response.json();

        // Проходим по всем select элементам и заполняем их
        const selects = form.querySelectorAll('select');
        selects.forEach(select => {
            // Ищем словарь по data-dict, если нет - по id (name, brand, color и т.д.)
            const dictKey = select.getAttribute('data-dict') || select.id;
            const dict = mappings[dictKey];

            if (dict) {
                // Очищаем "Загрузка..." и ставим дефолтный пункт
                select.innerHTML = '<option value="" disabled selected>Выберите значение</option>';
                
                // Получаем все ключи (названия) из словаря и сортируем их по алфавиту
                const options = Object.keys(dict).sort();
                
                options.forEach(optValue => {
                    const optionElement = document.createElement('option');
                    optionElement.value = optValue;
                    // Делаем первую букву заглавной для красоты
                    optionElement.textContent = optValue.charAt(0).toUpperCase() + optValue.slice(1);
                    select.appendChild(optionElement);
                });
            } else {
                select.innerHTML = `<option value="" disabled selected>Словарь ${dictKey} не найден</option>`;
            }
        });
    } catch (error) {
        console.error('Ошибка загрузки маппингов:', error);
        alert('Не удалось загрузить списки категорий из модели.');
    }

    // 2. Обработка отправки формы
    form.addEventListener('submit', async (e) => {
        e.preventDefault();
        form.classList.add('submitted');

        if (!form.checkValidity()) return;

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
