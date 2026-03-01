use axum::{Json, Router, extract::State, routing::post};
use ort::{inputs, session::Session};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

const E: f32 = 0.1571;
// 1. Описываем какой json мы ждем от пользователя,
// Макрос Deserialize заставляет serde автоматически парсить входящий json в структуру
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CarFeatures {
    pub year: f32,
    pub mileage: f32,
    pub power: f32,
    pub brand: String,
    pub name: String,
    pub body_type: String,
    pub color: String,
    pub fuel_type: String,
    pub transmission: String,
    pub vehicle_configuration: String,
    pub engine_name: String,
    pub engine_displacement: String,
    pub location: String,
    pub price: Option<f32>,
}
#[derive(Deserialize, Clone)]
pub struct CategoryMappings {
    pub brand: HashMap<String, u32>,
    pub name: HashMap<String, u32>,
    #[serde(rename = "bodyType")] // Сопоставляем camelCase из JSON с snake_case в Rust
    pub body_type: HashMap<String, u32>,
    pub color: HashMap<String, u32>,
    #[serde(rename = "fuelType")]
    pub fuel_type: HashMap<String, u32>,
    pub transmission: HashMap<String, u32>,
    #[serde(rename = "vehicleConfiguration")]
    pub vehicle_configuration: HashMap<String, u32>,
    #[serde(rename = "engineName")]
    pub engine_name: HashMap<String, u32>,
    #[serde(rename = "engineDisplacement")]
    pub engine_displacement: HashMap<String, u32>,
    pub location: HashMap<String, u32>,
}
// 2. Описываем какую структуру возвращаем пользователю
// Макрос Serialize запаковывает структуру обратно в json
enum DealQuality {
    Suspicious,
    Profitable,
    Normal,
    Expensive,
    TooExpensive,
}

#[derive(Serialize)]

pub struct Prediction {
    pub predicted_price: f32,
    pub verdict: String,
}

pub struct AppState {
    pub mappings: HashMap<String, HashMap<String, u32>>,
    pub model: Mutex<Session>,
}

fn encode_binary(id: u32, num_bits: usize, feature_vec: &mut Vec<f32>) {
    for i in 0..num_bits {
        let bit = if (id & (1 << i)) > 0 { 1.0 } else { 0.0 };
        feature_vec.push(bit);
    }
}

impl DealQuality {
    fn diff(d: f32) -> Self {
        match d {
            x if x < -2.0 * E => Self::Suspicious,
            x if x < -E => Self::Profitable,
            x if x <= E => Self::Normal,
            x if x <= 2.0 * E => Self::Expensive,
            _ => Self::TooExpensive,
        }
    }

    fn to_massage(&self) -> String {
        match self {
            Self::Suspicious => "Подозрительно: слишком дешево".to_string(),
            Self::Profitable => "Выгодно".to_string(),
            Self::Normal => "Нормально: рыночная цена".to_string(),
            Self::Expensive => "Дорого: выше оценки".to_string(),
            Self::TooExpensive => "Слишком завышенная цена".to_string(),
        }
    }
}

impl CarFeatures {
    pub fn to_features(&self, m: &CategoryMappings) -> Vec<f32> {
        let mut f: Vec<f32> = Vec::with_capacity(86);

        // Базовые числовые признаки
        f.push(self.year);
        f.push(self.mileage);
        f.push(self.power);

        // Вспомогательная функция для получения ID или 0 (default)
        let get_id = |map: &HashMap<String, u32>, key: &String| map.get(key).copied().unwrap_or(0);

        // Кодируем категориальные признаки в биты
        encode_binary(get_id(&m.brand, &self.brand), 8, &mut f);
        encode_binary(get_id(&m.name, &self.name), 12, &mut f);
        encode_binary(get_id(&m.body_type, &self.body_type), 4, &mut f);
        encode_binary(get_id(&m.color, &self.color), 5, &mut f);
        encode_binary(get_id(&m.fuel_type, &self.fuel_type), 2, &mut f);
        encode_binary(get_id(&m.transmission, &self.transmission), 3, &mut f);
        encode_binary(
            get_id(&m.vehicle_configuration, &self.vehicle_configuration),
            15,
            &mut f,
        );
        encode_binary(get_id(&m.engine_name, &self.engine_name), 13, &mut f);
        encode_binary(
            get_id(&m.engine_displacement, &self.engine_displacement),
            7,
            &mut f,
        );
        encode_binary(get_id(&m.location, &self.location), 12, &mut f);

        // Вычисляемые признаки
        let car_age = (2026.0 - self.year).max(1.0);
        f.push(car_age);
        f.push(self.mileage / car_age);

        f
    }
}

// 3. Обработчик (Handler). Он асинхронный (async)
// Json<CarFeatures> говорит axum: "Вытащи из тела запроса JSON и преврати в структуру
async fn price_prediction(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CarFeatures>,
) -> Json<Prediction> {
    let mut features: Vec<f32> = Vec::with_capacity(86);

    features.push(payload.year);
    features.push(payload.mileage);
    features.push(payload.power);

    let brand_id = state.mappings["brand"]
        .get(&payload.brand)
        .copied()
        .unwrap_or(0);
    let name_id = state.mappings["name"]
        .get(&payload.name)
        .copied()
        .unwrap_or(0);
    let body_type_id = state.mappings["bodyType"]
        .get(&payload.body_type)
        .copied()
        .unwrap_or(0);
    let color_id = state.mappings["color"]
        .get(&payload.color)
        .copied()
        .unwrap_or(0);
    let fuel_type_id = state.mappings["fuelType"]
        .get(&payload.fuel_type)
        .copied()
        .unwrap_or(0);
    let transmission_id = state.mappings["transmission"]
        .get(&payload.transmission)
        .copied()
        .unwrap_or(0);
    let vehicle_configuration_id = state.mappings["vehicleConfiguration"]
        .get(&payload.vehicle_configuration)
        .copied()
        .unwrap_or(0);
    let engine_name_id = state.mappings["engineName"]
        .get(&payload.engine_name)
        .copied()
        .unwrap_or(0);
    let engine_displacement_id = state.mappings["engineDisplacement"]
        .get(&payload.engine_displacement)
        .copied()
        .unwrap_or(0);
    let location_id = state.mappings["location"]
        .get(&payload.location)
        .copied()
        .unwrap_or(0);

    encode_binary(brand_id, 8, &mut features);
    encode_binary(name_id, 12, &mut features);
    encode_binary(body_type_id, 4, &mut features);
    encode_binary(color_id, 5, &mut features);
    encode_binary(fuel_type_id, 2, &mut features);
    encode_binary(transmission_id, 3, &mut features);
    encode_binary(vehicle_configuration_id, 15, &mut features);
    encode_binary(engine_name_id, 13, &mut features);
    encode_binary(engine_displacement_id, 7, &mut features);
    encode_binary(location_id, 12, &mut features);

    let car_age = (2026.0 - payload.year).max(1.0);
    let mileage_per_year = payload.mileage / car_age;

    features.push(car_age);
    features.push(mileage_per_year);

    // 1. Создаем тензор напрямую из вектора
    let tensor =
        ort::value::Value::from_array(([1, 86], features)).expect("Ошибка создания тензора");

    // 2. Блокируем мьютекс (ждем, если модель занята другим запросом)
    let mut model_lock = state.model.lock().await;

    // 3. Делаем инференс! Передаем разблокированную модель.
    // Если имя входа не "X", он тут упадет с ошибкой и подскажет правильное имя
    let out = model_lock
        .run(inputs!["float_input" => tensor])
        .expect("Ошибка инференса");

    // 4. Достаем число
    let pred_log: f32 = out[0].try_extract_tensor::<f32>().unwrap().1[0];
    // 4. Возвращаем цену (обратная операция логарифму: exp(x) - 1)
    let predicted_price = pred_log.exp() - 1.0;

    // Простая бизнес-логика
    let verdict = match payload.price {
        // Сценарий 1: Цена есть -> Считаем выгоду
        Some(user_price) => {
            let d = (user_price / predicted_price) - 1.0;
            DealQuality::diff(d).to_massage()
        }
        // Сценарий 2: Цены нет -> Просто отдаем информацию
        None => "Цена не указана. Используйте предсказание для ориентира.".to_string(),
    };
    Json(Prediction {
        predicted_price,
        verdict,
    })
}

// 4. Точка входа
#[tokio::main] // Этот макрос запускает асинхронный движок
async fn main() {
    ort::init().with_name("GoodDeal").commit();

    println!("Загрузка ИИ модели...");
    // 2. Загружаем саму модель из файла
    let model = Session::builder()
        .unwrap()
        .commit_from_file("model/random_forest_cars.onnx")
        .expect("Не найден файл модели!");

    println!("Загрузка маппингов из Json");

    let mapping_data =
        std::fs::read_to_string("data/category_mappings.json").expect("Can not read JSON");
    let mappings: HashMap<String, HashMap<String, u32>> =
        serde_json::from_str(&mapping_data).expect("JSON Parsing error");

    println!("Successfully loaded dictionaries: {}", mappings.len());

    let shared_state = Arc::new(AppState {
        mappings,
        model: Mutex::new(model),
    });

    let app = Router::new()
        .route("/predict", post(price_prediction))
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Сервер запущен на http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
