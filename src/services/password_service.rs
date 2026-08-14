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

// ================================
//              TESTI
// ================================
#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password}; // uvozimo iz nadrejenega modula
    use argon2::password_hash::PasswordHash;
    
    #[test]
    fn hash_password_naredi_ustrezen_hash() {
        let password = "varnogeslo123";

        let hash = hash_password(password);

        assert_ne!(hash, password); // nista enaka
        assert!(PasswordHash::new(&hash).is_ok());
    }

    #[test]
    fn verify_password_sprejme_pravilno_geslo() {
        let password = "pravilnogeslo123";
        let hash = hash_password(password);

        assert!(verify_password(password, &hash));
    }

    #[test]
    fn verify_password_zavrne_napacno_geslo() {
        let password = "pravilnogeslo123";
        let hash = hash_password(password);

        assert!(!verify_password("napacnogeslo123", &hash))
    }

    #[test]
    fn verify_password_zavrne_neveljaven_hash() {
        assert!(!verify_password("varnogeslo123", "neveljavenhash"))
    }

    #[test]
    fn isto_geslo_da_druge_hashe() {
        let password = "varnogeslo123";

        let first_hash = hash_password(password);
        let second_hash = hash_password(password);

        assert_ne!(first_hash, second_hash);
        assert!(verify_password(password, &first_hash));
        assert!(verify_password(password, &second_hash));
    }
}