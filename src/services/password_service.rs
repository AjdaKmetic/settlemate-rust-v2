use argon2::{
    Argon2,
    password_hash::{
        PasswordHash,
        PasswordHasher,
        PasswordVerifier,
        SaltString,
    },
};

use rand_core::OsRng;

// ustvari hash gesla
pub fn hash_password(password: &str) -> String {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Hashiranje gesla ni uspelo")
        .to_string()
}

// preveri, ali se geslo ujema s shranjenim hashem
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = PasswordHash::new(password_hash);

    match parsed_hash {
        Ok(hash) => Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok(),

        Err(_) => false,
    }
}