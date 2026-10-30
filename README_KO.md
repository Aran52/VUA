# VUA — Virtual Unified Assistant

[English](README.md) | [简体中文](README_ZH.md) | [日本語](README_JA.md) | 한국어

**VUA(Virtual Unified Assistant)** 는 VRChat 플레이어를 위한 Windows 우선·로컬 우선 시나리오 설치·관리·실행
앱입니다. 시나리오는 목표, 필요한 소프트웨어, 준비·실행·복구 단계를 묶은 것입니다. PC 화면으로 플레이하기,
PICO로 스트리밍하기, 선택한 모듈로 Avatar 편집하기처럼 하고 싶은 일부터 다음 단계를 안내합니다.

## VUA로 할 수 있는 일

### 1. 게임 도우미

> **Materials checked and cleared.**

플레이에 필요한 소프트웨어와 설정을 준비하고 이동, 메뉴, 안전 설정, 기기 사용의 기초를 배웁니다. 사용 목적과 하드웨어에 맞춰 필요한 항목을 설명하고 환경 설치를 안내합니다.

첫 계정 안내는 내장 브라우저에서 공식 가입 페이지를 엽니다. 플레이 경로는 Steam과 VRChat, 제작 경로는 선택적으로 Unity와 BOOTH를 안내합니다. 가입, Steam 라이브러리 추가 및 공식 계정 연결은 사용자가 직접 수행합니다. 업로드에는 정식 VRChat 계정과 New User 이상 등급이 필요하지만, 자격을 얻기 전에도 로컬 Avatar 제작 준비를 진행할 수 있습니다.

선택적 외부 연결은 추적, VR 오버레이, 수면 보조, 녹화 및 방송 도구를 대상으로 합니다. VUA는 독립적으로 설치된 앱을 감지하고 Steam 구매·설치 화면을 안내하며 앱을 실행할 예정입니다. 업데이트는 Steam 또는 업스트림 앱이 관리합니다. 추적 도구에는 설정 안내도 제공하며, PICO 시선 추적은 첫 플레이용 릴리스에 포함합니다. [대상 도구와 지원 범위](docs/development-outline.md#n2-external-gameplay-tools)를 참고하세요.

### 2. Avatar 제작

> **Sugar, spice, and everything nice.**

AMF는 VUA와 함께 설치되는 자체 편집 모듈이며 새 사용자에게 기본으로 켜져 있습니다.
아바타 편집 또는 「설정 → 기능 모듈」에서 끌 수 있고 기존 선택은 기억됩니다. 에셋 라이브러리와 전용 BDL은 AMF가 관리하며 꺼도 데이터는 유지됩니다.

보유한 소재를 조합하고 선택과 설정을 Recipe에 기록해 공유하며, 다른 사용자는 직접 구한 소재로 이를 재현합니다. 목표는 검사와 통제된 변경을 거쳐 Avatar를 자동 제작하고 결과물을 공식 SDK에 전달하는 것입니다. Recipe에는 참조와 설정만 담으며 유료 소재 자체는 포함하지 않습니다.

## 사용 흐름

목표와 보유한 소재에서 시작하여 계획, 실행 단계와 결과를 확인합니다. Build Record는 제작 단계, 검사 근거와 오류를 모아 수행한 작업과 조치가 필요한 부분을 보여 줍니다.

프로젝트 및 패키지 관리는 Unity 환경과 의존성, VPM 저장소, 패키지 설치·업데이트·제거를 준비합니다. 기존 ALCOM/VCC 프로젝트는 읽기 전용으로 검사하고, 편집은 사용자가 요청해 만든 VUA 관리 사본에서 진행합니다. 최종 로그인과 업로드는 사용자가 VRChat 공식 SDK에서 직접 수행합니다.

## 안전 경계

- VUA는 VRChat Inc.와 제휴하거나 공식 승인을 받은 제품이 아닌 독립적인 서드파티 도우미이며, 문서화된 외부 인터페이스, OSC, 실행 옵션, 필요한 로컬 로그와 문서화된 설정만 사용하고 VRChat 클라이언트 주입·후킹·패치 또는 EAC 우회를 하지 않습니다.
- 첫 계정 안내는 격리된 임시 브라우저 세션을 사용합니다. VUA 앱 기능은 비밀번호를 수집하거나 로그인 Cookie와 토큰을 앱 데이터, 로그 또는 에이전트 컨텍스트로 추출하지 않으며, 첫 제공 범위에서는 VRChat 로그인을 기억하지 않습니다. 이후 웹 정보 읽기와 실험적 로그인 유지 기능은 별도 개발 방향이며 플랫폼 승인을 뜻하지 않습니다. [계정 경계](docs/product-boundary.md#account-onboarding-user-ruling-2026-09-30)를 참조하세요.
- 계정 변경은 사용자가 허용된 절차로 시작해야 하며, VUA가 클라우드에서 계정을 대신 조작하거나 Avatar를 자동으로 대신 업로드하지 않습니다.
- 기능에 필요한 최소 데이터만 기본적으로 로컬에 보관하고 불필요한 친구 활동 추적이나 프로파일링을 만들지 않으며, 유료 소재는 로컬에 두고 공유 Recipe에는 소재 자체를 넣지 않습니다.
- 문서화되지 않은 클라이언트 동작, 숨겨진 설정 및 통제되지 않는 API 자동화는 기본 범위에서 제외되며, 기술 검사 통과는 외형·동작·실사용 안전성을 보장하지 않습니다.

[VRChat Creator Guidelines](https://hello.vrchat.com/creator-guidelines) · [Configuration File](https://docs.vrchat.com/docs/configuration-file)

## 개발 진행 상황

[v0.7.0 Stone Age(Beta)](https://github.com/VUA-Project/VUA/releases/tag/v0.7.0)는 처음으로 아바타 제작 흐름을 제공합니다. 라이브러리에서 아바타와 의상을 고르면 VUA가 전용 Unity 프로젝트에서 조립하고 VRChat SDK로 확인한 뒤, 확인을 거쳐 업로드합니다. Windows x64 온라인·오프라인 NSIS 설치 파일에 AMF가 포함됩니다. [N1–N7 개발 순서](docs/development-outline.md)에 따라 개발을 이어 갑니다.

위 다운로드는 공개된 0.7.0 Beta입니다. 소스에서는 **v1.0.0 Brave New World**를 준비하고 있어요. 본체는 안정 버전이 되고 AMF는 Beta로 독립 업데이트를 이어 갑니다. [동결 버전 릴리스 노트 초안](docs/release/v1.0.0_KO.md)에 범위와 작성자 확인 대기 항목을 적었습니다. 소스 준비만으로 설치 파일이 공개되거나 N 단계의 검수가 끝나는 것은 아니에요.

### 현재 제공 상태와 시작 방법

- [공식 다운로드 페이지](https://ph-r.com/projects/vua/#download) 또는 [GitHub 릴리스](https://github.com/VUA-Project/VUA/releases/tag/v0.7.0)에서 **VUA-0.7.0-windows-x64-setup.exe**를 내려받습니다. 다운로드가 불안정하면 **VUA-0.7.0-windows-x64-offline.exe**를 선택해 주세요. 두 곳 모두 같은 파일을 제공합니다. 설치 후 시작 메뉴에서 VUA를 엽니다. Node, pnpm, Rust나 소스는 필요하지 않습니다. 서명되지 않은 빌드이므로 Windows가 알 수 없는 게시자 경고를 표시할 수 있습니다.
- 첫 실행 기능 안내와 가이드로 시작하거나 플레이, 도구, Avatar 편집을 바로 엽니다. 새 사용자는 AMF가 활성화되며 설정 → 모듈에서 끌 수 있습니다. Steam/PICO/Unity 및 계정 작업은 공식 앱에서 완료합니다.
- VUA 업데이트 전 작업을 마치고 앱을 종료한 다음 새 설치 파일을 실행합니다. AMF는 설정 → 모듈 또는 정보에서 따로 업데이트한 뒤 명시적으로 다시 시작할 수 있습니다. 데이터는 %APPDATA%\VUA에 유지되며 ZIP 버전에서 설치 버전으로 옮겨도 같은 프로필을 사용합니다. 미검증 과정과 수동 인계는 릴리스 노트에 기록하며 문제는 [Issues](https://github.com/VUA-Project/VUA/issues/new/choose)에 남길 수 있습니다.
- 개발자는 [의존성 설치, 앱 실행 및 검사](apps/desktop/README.md#development-commands)와 [기여 가이드](CONTRIBUTING.md)를 따릅니다.

## 문서

- [문서 가이드](docs/README.md)——작업별 최소 경로
- [제품 경계](docs/product-boundary.md)
- [아키텍처](docs/architecture/system.md)
- [v0.7.0 릴리스 노트](docs/release/v0.7.0_KO.md)
- [1.0.0 최소 안내서(후보판, 다섯 언어)](docs/user-guide/v1.0.0/ko.md)
- [기여 가이드](CONTRIBUTING.md) · [보안 정책](SECURITY.md)

## 지나온 길
날짜는 각 릴리스와 풀 리퀘스트에 대해 GitHub가 기록한 시각입니다.

| 날짜 | 마일스톤 |
| --- | --- |
| 2026-09-07 | [v0.5.0](https://github.com/VUA-Project/VUA/releases/tag/v0.5.0): 데스크톱 앱에서 Rust 오케스트레이터를 거쳐 실제 Unity 2022.3 프로젝트까지 이어져, 아바타에 의상 한 벌을 조립 |
| 2026-10-01 | [목적에 맞춘 플레이 환경 준비](https://github.com/VUA-Project/VUA/pull/60) (지역별 Unity 다운로드 경로 포함) |
| 2026-10-08 | [BOOTH 라이브러리 동기화, 선택 다운로드, 로컬 파일 관리](https://github.com/VUA-Project/VUA/pull/66) |
| 2026-10-11 | [v0.6.6 Ibis](https://github.com/VUA-Project/VUA/releases/tag/v0.6.6): 첫 공개 베타, AMF를 내장 모듈로 포함 |
| 2026-10-12 | [NSIS 설치 프로그램과 AMF 개별 업데이트](https://github.com/VUA-Project/VUA/pull/80) |
| 2026-10-15 | [소재 중심 아바타 제작, Recipe, 로컬 MCP 연결](https://github.com/VUA-Project/VUA/pull/84) |
| 2026-10-17 | [60프레임 턴테이블과 관리형 업로드를 갖춘 릴리스 룸](https://github.com/Aran52/VUA/pull/4) |
| 2026-10-18 | [VN3 약관 해석과 용도별 검사](https://github.com/Aran52/VUA/pull/31) |
| 2026-10-22 | [v0.7.0 Stone Age](https://github.com/VUA-Project/VUA/releases/tag/v0.7.0): 아바타 제작 흐름을 담은 첫 릴리스 |

## 라이선스

VUA는 [Apache-2.0](LICENSE)을 사용합니다. 외부 앱에는 각각의 라이선스가 적용되며, 여기에는 [VRCFaceTracking(Apache-2.0)](https://github.com/benaclejames/VRCFaceTracking/blob/master/LICENSE)과 [Space Calibrator(MIT 본체 및 별도 라이선스의 서드파티 구성 요소)](https://github.com/hyblocker/OpenVR-SpaceCalibrator/blob/develop/LICENSE)도 포함됩니다. 지원 예정인 모든 외부 연결, 저작권과 향후 재배포 조건은 [서드파티 고지](THIRD_PARTY_NOTICES.md)를 참조하세요.

[NOTICE](NOTICE) · [상표 가이드](TRADEMARKS.md)

Copyright 2026 Aran52.
