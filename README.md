# SettleMate

SettleMate je spletna aplikacija za beleženje in deljenje skupnih stroškov. Uporabnikom omogoča dodajanje prijateljev, ustvarjanje skupin, razdelitev stroškov med več oseb ter pregled stanja in preteklih aktivnosti.

## Funkcionalnosti

- registracija, prijava in odjava uporabnika,
- sprememba imena in gesla uporabniškega računa,
- dodajanje prijateljev,
- ustvarjanje skupin in dodajanje njihovih članov,
- zapustitev in brisanje skupine,
- dodajanje stroška z več izbranimi uporabniki,
- dodajanje stroška za vse člane izbrane skupine,
- samodejna enakomerna razdelitev stroška,
- poravnava dolgov med uporabniki,
- pregled trenutnega stanja in aktivnosti,
- prikaz podrobnosti stroška,
- sprememba opisa in brisanje stroška.

Pri ustvarjanju stroška je prijavljeni uporabnik vedno določen kot plačnik.

## Namestitev in zagon

### Predpogoji

Za zagon potrebuješ:

- Git,
- Rust in Cargo.

### 1. Kloniranje repozitorija

```bash
git clone https://github.com/jo53289/settlemate-rust-v2.2.git
cd settlemate-rust-v2.2
```

### 2. Priprava baze

V korenski mapi projekta ustvari datoteko SQLite baze:

```bash
touch settlemate.db
```

Nato ustvari datoteko `.env`:

```bash
touch .env
```

V datoteko `.env` zapiši:

```env
DATABASE_URL=sqlite://settlemate.db?mode=rwc
```

### 3. Zagon migracij

Tabele v podatkovni bazi ustvari z ukazom:

```bash
cargo run --manifest-path migration/Cargo.toml -- up
```

### 4. Zagon aplikacije

```bash
cargo run
```

Aplikacija bo dostopna na naslovu:

```text
http://127.0.0.1:3000
```

## Uporaba aplikacije

1. Uporabnik najprej ustvari račun ali se prijavi z obstoječim računom.
2. Na zavihku **Prijatelji** lahko doda druge uporabnike.
3. Na zavihku **Skupine** lahko ustvari skupino in vanjo doda člane.
4. Pri dodajanju stroška lahko izbere:
   - deljenje z več posameznimi uporabniki ali
   - deljenje z vsemi člani izbrane skupine.
5. Prijavljeni uporabnik je samodejno določen kot plačnik, znesek pa se enakomerno razdeli med vse vključene osebe.
6. Na zavihku **Aktivnost** lahko pregleda stroške, njihove podrobnosti, spremeni opis ali strošek izbriše.
7. Če ima uporabnik dolg do druge osebe, ga lahko poravna. Poravnava se zabeleži in upošteva pri izračunu trenutnega stanja.
8. Prek strani **Moj račun** lahko spremeni svoje ime ali geslo in se odjavi.

## Testiranje

Teste zaženeš v korenski mapi projekta:

```bash
cargo test
```
