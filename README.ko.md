# mnf

![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-green.svg)
![Interface](https://img.shields.io/badge/Interface-CLI%20%2B%20TUI-blue)

`mnf`는 마인크래프트 닉네임을 찾기 위한 Rust 기반 터미널 앱입니다.

스타일이 적용된 CLI 모드와 상호작용 가능한 TUI 모드를 모두 지원하며, 길이와 접두사를 기준으로 검색하고, 찾은 결과를 텍스트 또는 CSV 파일로 저장할 수 있습니다.

중요: 이 프로젝트는 공개 Mojang 프로필 조회를 사용하므로 결과는 `likely available`로 표시되며, 실제 등록 가능 여부를 완전히 보장하지는 않습니다.

## 주요 기능

- `3`~`10` 글자 길이로 닉네임 검색
- `e`, `ab`, `mc_` 같은 시작 문자 또는 접두사 지정
- 두 가지 실행 방식 제공:
  - 예쁘게 꾸며진 CLI 모드
  - 실시간 상태를 보여주는 TUI 모드
- 후보를 랜덤 순서로 중복 없이 탐색
- 진행 상황, 발견 개수, 배치 수, 종료 사유 표시
- `--save` 옵션으로 CLI 결과를 텍스트 또는 CSV로 저장
- CLI와 TUI가 동일한 검색 엔진을 공유하는 단일 Rust 바이너리 구조

## 빠른 시작

### 준비물

- `cargo`를 포함한 Rust 툴체인

### 빌드

```bash
cargo build
```

### 로컬 설치

```bash
cargo install --path .
```

설치 후에는 바이너리를 바로 실행할 수 있습니다.

```bash
mnf cli --length 4 --starts-with e --results 3 --max-checks 20
mnf tui
```

### CLI 실행

```bash
cargo run -- cli --length 4 --starts-with e --results 3 --max-checks 20
```

이 모드는 스타일이 적용된 헤더, 진행 표시, 강조된 결과, 최종 요약을 출력합니다.

### 파일로 저장하기

```bash
cargo run -- cli --length 4 --starts-with e --results 10 --max-checks 200 --save names.txt
```

```bash
cargo run -- cli --length 4 --starts-with e --results 10 --max-checks 200 --save names.csv
```

- `.csv`가 아닌 경로는 기본적으로 plain text로 저장됩니다
- `.csv` 경로는 `name` 헤더가 포함된 CSV로 저장됩니다

### TUI 실행

```bash
cargo run -- tui
```

기본 TUI 값:

- `length = 4`
- `prefix = ""`
- `results = 10`
- `max_checks = 200`

## CLI 옵션

`mnf`는 두 개의 서브커맨드를 제공합니다.

- `cli` - 명령줄에서 한 번 검색 실행
- `tui` - 대화형 터미널 UI 실행

CLI 플래그:

- `--length <u8>` - 목표 닉네임 길이
- `--starts-with <text>` - 선택 접두사
- `--results <usize>` - 찾고 싶은 `likely available` 결과 개수
- `--max-checks <usize>` - 검사할 후보 최대 개수
- `--save <path>` - 선택 결과 저장 경로

예시:

```bash
cargo run -- cli --length 5 --starts-with mc --results 20 --max-checks 300 --save mc-names.csv
```

## 예시 명령

### `e`로 시작하는 4글자 닉네임 찾기

```bash
cargo run -- cli --length 4 --starts-with e --results 10 --max-checks 200
```

### 접두사 없이 랜덤 6글자 닉네임 찾기

```bash
cargo run -- cli --length 6 --results 15 --max-checks 400
```

### 결과를 CSV로 많이 저장하기

```bash
cargo run -- cli --length 5 --starts-with mc --results 50 --max-checks 1000 --save mc-names.csv
```

### 결과를 텍스트 파일로 저장하기

```bash
cargo run -- cli --length 4 --starts-with a --results 25 --max-checks 500 --save names.txt
```

### 초기값을 지정해서 TUI 열기

```bash
cargo run -- tui --length 5 --starts-with mc --results 25 --max-checks 400
```

## TUI 조작법

TUI 내부에서는 다음 키를 사용합니다.

- `Enter` - 검색 시작 또는 중지
- `Tab`, 방향키 - 입력 필드 이동
- 타이핑 - 대기 상태에서 선택된 필드 수정
- 검색 중에는 입력 수정이 잠깁니다
- `q` 또는 `Esc` - 종료

## 검색 동작

- 지원하는 길이는 `3..=10`
- 허용 문자는 `A-Z`, `a-z`, `0-9`, `_`
- 접두사는 목표 길이보다 길 수 없습니다
- 한 번의 검색 실행 안에서는 후보가 랜덤 순서로 중복 없이 생성됩니다
- 결과는 공개 Mojang 프로필 조회를 기반으로 계산됩니다
- 공개 조회는 완전한 등록 가능 판정이 아니므로 결과는 `likely available`로 표시됩니다

## 프로젝트 구조

```text
.
├── Cargo.toml
├── LICENSE
├── README.md
├── README.ko.md
├── docs/
│   └── superpowers/
│       └── plans/
│           └── 2026-03-24-minecraft-name-finder.md
└── src/
    ├── checker.rs
    ├── cli.rs
    ├── generator.rs
    ├── lib.rs
    ├── main.rs
    ├── model.rs
    ├── output.rs
    ├── search.rs
    ├── tui/
    │   └── mod.rs
    └── validation.rs
```

## 모듈 개요

- `src/main.rs` - 엔트리 포인트와 서브커맨드 분기
- `src/cli.rs` - 스타일이 적용된 CLI 흐름과 `--save` 처리
- `src/tui/mod.rs` - 대화형 터미널 대시보드
- `src/search.rs` - 공통 검색 루프, 진행 이벤트, 종료 조건
- `src/checker.rs` - Mojang 조회 클라이언트와 재시도/폴백 처리
- `src/generator.rs` - 랜덤 순서의 중복 없는 후보 생성
- `src/validation.rs` - 길이, 접두사, 옵션 검증
- `src/model.rs` - 공통 검색 모델, 요약, 이벤트 타입
- `src/output.rs` - 텍스트/CSV 저장 헬퍼

## 기술 스택

- Rust 2024 edition
- 비동기 실행을 위한 `tokio`
- Mojang API 요청/파싱을 위한 `reqwest` + `serde`
- CLI 파싱을 위한 `clap`
- 터미널 UI를 위한 `ratatui` + `crossterm`
- 꾸며진 CLI 출력을 위한 `indicatif` + `owo-colors`
- 랜덤 후보 생성을 위한 `rand`
- 에러 처리를 위한 `anyhow`

## 개발

주요 로컬 검증 명령:

```bash
cargo fmt
cargo test
cargo check
```


## 테스트

현재 테스트는 다음 내용을 다룹니다.

- 입력 검증
- 랜덤 후보 생성
- 결과 저장 포맷
- Mojang 응답 분류
- 공통 검색 종료 조건과 취소 동작

전체 테스트 실행:

```bash
cargo test
```

## 기여 가이드

프로젝트를 확장할 때는 다음 원칙을 유지하는 것이 좋습니다.

- 조회 소스가 더 권위 있게 바뀌지 않는 한 `likely available` 표현 유지
- CLI와 TUI는 같은 검색 엔진 위에서 동작하도록 유지
- UI, 네트워크, 검증 로직을 한 파일에 섞지 말고 역할을 분리
- 변경 후 `cargo fmt`, `cargo test`, `cargo check` 실행

## 라이선스

이 프로젝트는 MIT 라이선스를 따릅니다. 자세한 내용은 `LICENSE`를 참고하세요.
