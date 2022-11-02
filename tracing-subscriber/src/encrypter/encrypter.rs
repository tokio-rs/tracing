pub trait Encrypter {
    fn encrypt(&self, msg: String) -> String;
}

#[derive(Debug)]
pub struct DefaultEncrypter;

impl Encrypter for DefaultEncrypter {
    fn encrypt(&self, msg: String) -> String {
        base64::encode(msg)
    }
}

#[derive(Debug)]
pub struct TestEncrypter;

impl Encrypter for TestEncrypter {
    fn encrypt(&self, msg: String) -> String {
        msg.as_bytes()
            .into_iter()
            .map(|c| c.wrapping_add(20) as char)
            .collect()
    }
}

#[test]
fn test_shift_encrypter() {
    let t = TestEncrypter;
    let st = t.encrypt("helloworld".to_string());
    println!("{st}");
}

#[test]
fn test_default_encrypter() {
    use core::str::from_utf8;

    let t = DefaultEncrypter;
    let st = t.encrypt("helloworld\n".to_string());
    println!("{st}");
    let dest = base64::decode(st).unwrap();
    let st = from_utf8(dest.as_slice()).unwrap();
    println!("{st}");
}
