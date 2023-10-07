use std::sync::Mutex;

static mut DB: Option<sled::Db> = None;
static DB_MUTEX: Mutex<()> = Mutex::new(()); 

pub fn get_db() -> &'static sled::Db {
  let _lock = DB_MUTEX.lock().unwrap();
  unsafe {
    if DB.is_none() {
      DB = Some(sled::open("/tmp/db").unwrap());
    }
    DB.as_ref().unwrap()
  }
}