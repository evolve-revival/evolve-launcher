use axum::{
    Router,
    body::Body,
    extract::Path,
    http::Request,
    middleware::Next,
    response::Response,
    routing::{get, post},
    Json,
};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const APP_GROUP_ID: &str = "c3dc178f670ee769fe59e244610d66e2";

pub const ENTITLEMENT_IDS: &[&str] = &[
    "c7d22439bc13e53554776bbec4c175db",
    "bada12bfd30bcd02a609f7c88ca0a244",
    "d732cbb899e12be62d1fdfb6d43ac15c",
    "c0e9412ae0e437935f4fc1ef9db8add7",
    "47b1ec72d6dee517cc7fd896e5c66c93",
    "79c0b7fbb8b5a610d5665fe66e3c6c6a",
    "2ebd108f2cdce9e5315295786859eabe",
    "7ea5435a8af71001b790a6766d0b14b5",
    "5511deee62056a54d1a940fc8d14d1d7",
    "bb9e6744eb44a96c25987283f2150844",
    "034463e6f628e94147c366850693f3af",
    "ae60dc2f1dc7ce5e837e23d765308a7f",
    "09dc8569d41d18169dba133b646a0c7c",
    "ae16a01bd0f92671190a36f7edcd218c",
    "f26c3f0e0d4c89f9583403d08c68fb15",
    "669261b069a12f196ca6e083c5944059",
    "92f875f36cf4fe94265d07ac1574678b",
    "cb4b79d5d1e10b0b0a3dc98f044c0f14",
    "4b3b9650cf729c8ebd90354d22121f5e",
    "c5bed6ded5991a7d5deb29df9e270856",
    "43745436a426991ef359ac767af529bf",
    "5e83883ab63364d58502733b10186c31",
    "c816eb19a5a20bbc035fed633a4c04ca",
    "cf6ff1e376dec1a317459794d60d916a",
    "5d8e4486551dcd54b27224a159e8885d",
    "9c84c670e462589c20e5cf95afea516a",
    "7ad74999061ae851075957b3b65d3f66",
    "a96e8e5e664d6d6fd230731d59d62e94",
    "e946877ca525e8379d9929de9c4e5890",
    "f3deb191029a5baee0e980b8d3e02894",
    "08fa249c59e21f042ed94ea43fc89388",
    "66d925a3256578604d6d8781f2e6f334",
    "3e8a5657d23aaa3b3b5bc34177e0f304",
    "bd45a1ac30c9e5cfd8987ecdc697cbc7",
    "68a2f02659a50aa5909aa7b8f1620a89",
    "0058bc66005652c37be48eca789a5958",
    "4bed036f7af351e916f6927b11e1ebcc",
    "82e97bf5c6180f7ee4b1e2f47ae283cf",
    "a8192b209767b53b745c7c3df2e9bdd6",
    "a39d9efb7c2f14e40ae6300167864d05",
    "11750c5fe007e9f44c22d8ef3c0465f0",
    "d7afd9d4bda84f400b162a02b1e2d345",
    "22d76280b995188e8a437a74be6e8807",
    "a761377c795b72765720838287928b86",
    "9d5f982d5e1ba5bdc14557257d5308ed",
    "912639e63331238969ffe5bed75883c9",
    "65eb48186ddc9447edfeaf902394f271",
    "f4bb97a5f9720b85f48ba548759c4554",
    "06541b9fb1369534d5094687e1aa45c2",
    "2a106881cf24251097ab08263c6fc493",
    "80bbe53b851874d5579aa9f91b19a263",
    "6ccf1271625a1fb889d3798e73434a9f",
    "9315bcb0313f247818331e2be37a3298",
    "5b4334700d5247c56c01613604f2e749",
    "3932addfb524e5c77c55411211ef752f",
    "c6498a81a0e35857ad9de2f6db858e9f",
    "a9bfd12867710569f31d0cf4e6cdb01b",
    "e865d77fdeebabdc16dceadd8aa02130",
    "3727ba1ef0514864225817e6f3ab4388",
    "af3695af22b7d823e24033016f5932e0",
    "afb22d3ac69443e827b305886c098730",
    "b23407aa52871b0d87821bb623d87dd0",
    "dd8d1e204b0001f9c0c473e0adc50c9b",
    "4a78664f20412551d946221ca7e3c5e8",
    "9435949aad0a0a2c893cd15506d590b3",
    "1d92afb2fe6d861b718246a8ab93b404",
    "b1fc47876643009f332591a11b435fbf",
    "9ba5600711ea147023dd16b1c497997c",
    "9256afea838ebb9cb3e8f006185d64e5",
    "f87902a41636d8a7e9da3b15fade94e8",
    "ad10bd86cf0505f2737cec9881181e56",
    "772542cfb05f541bff58102ed1ad7e54",
    "721a6c44b3d4389fd0012c273cdbce08",
    "1d255eb891731af82432d9ad06543dc4",
    "6435e760d4cf604ae969aa309ac426c1",
    "d7c2070109f89bfb844e3f4edbee6020",
    "73967d84117e66052d20210e30046625",
    "b316d8c399580f89d5bff34fea94db20",
    "a696a490fbab28bf8fa3a89cc739fa12",
    "f635290c4e4dbc0fd7799a09d349ea06",
    "a48237ad5874723ac3c84d25368bb502",
    "8d5096ce6370bf4d7ea9818c8349b416",
    "b4390d81791797a8ff9a18376e523a0a",
    "4dc0f1f9edd50085d5cce3b29d37d5c2",
    "23274e2f3c8c362911659645581110bb",
    "64fa1152317c16a0c2ebbab0bd095a60",
    "afd88c9264665b901ae7340c66fa3db3",
    "c8a23c241fb918769ee37bdc40978cea",
    "59dfea3e593b72acb2e6a85f429a5727",
    "7d9e77a8947fbe8df47871067c4334cd",
    "fef25e904ebdfefd286506521a1081bd",
    "772039dbc3169c655a25eb0795374e3a",
    "5a9213d12fb506f2571fd6c9448dae03",
    "30ebba69df93da13f623d0a0a1ced92a",
    "d86865d1fb8c8af334956c6cb11ca7d1",
    "aa8e56b99690be3351de882e7195189d",
    "b0fa38b8f04f93f631a25cd625a594d9",
    "a39e83b1b60aa26c238976b5efae9dba",
    "d88f675c5f2c137f9c230846726bf406",
    "267a825a2b27638adfaf6aa903123a3e",
    "bb13b25a2f3f0d1d9a6c24f1a357aa2b",
    "759d40cce5420473f70bb89259e85782",
    "ff7a88fd511aa7c4d357d44be2cff80f",
    "3382d4fc7492a912fe457c87662fa566",
    "5e2a694d14b6ac284a54be34a69c4caf",
    "e24f25246bbe5f4add4bc63300140b76",
    "97b37e27e15f6cc5e884226a86ffe982",
    "e1b5a3a3daa0eea254fb068f37ee5fe9",
    "03858494a073e9f0e741579ffd9e77f4",
    "6e6a745928f081c5c9b34069ad0cee16",
    "5b065960af2884d5ead4fa654cc82a3e",
    "ad440ba9a921a3007c6316359dcb4058",
    "13ebd9d0bb49ab407de91c4cbd78e4f7",
    "9f53b9a80db07c1fea69e3ec4bb25e5b",
    "ee2eef13716cb3e68a2f385a90bde3ff",
    "b9837511ad69b0ff4b010ab40a01ce73",
    "3176c7b3151d1bfed98fdd8e1d16864a",
    "f52739777523219c65978cc6ec070a1d",
    "a928790330759353df1daf1d4def010b",
    "7e947eff5fa796351c2f1c1814fe8838",
    "953605229bd608d01a68611994a92e31",
    "12e9e322ce2f4475cf180195d8a247cd",
    "347a63d1cb4c8a2ffc4a414c4a30b7f8",
    "e8757f1deab2ba666e5bde2811e527e6",
    "16bf3583a725e5f01dc32f26fd7e42cd",
    "bbda5e0c33e101a526b2f859f1a7bcea",
    "8905f7a981fb7b28268ab5cd085af42d",
    "e10de085526802808a5861077263b2f0",
    "7bd06d7cb120dad11891ea4b50415594",
    "8b5b05762cf5d15ee124b731980cb108",
    "535cc2d1a2435808216d0323da61ca9a",
    "41afb8a5bf795e30e7de80ec295bfde6",
    "59cd3ac994054e52e20f46b7903830c6",
    "2b079c0708cd3fe9e112069cefa13a77",
    "9c5934e1bdcb0abddae5636be15a3c05",
    "cdeaeaeefb8255559726e479886f1d6e",
    "802f90a905bab89d072b99ac8b4d4c8c",
    "f44ce54490c9679425011b91c246923a",
    "5b3b688707fe08252f6a8199fef2e85a",
    "c0c584524ffc2703c23ff889d250a8c4",
    "86989fb8de34a315baac82b54a88dfc9",
    "5bec8deb438bf7082e6bb40ff23d4a71",
    "b4dacdb14eb18491ae5ad43a9a1ac574",
    "3f7c6994a6efe913816188bb02463aa5",
    "26b654d35a06729e252b057e23c6a6e9",
    "93c6f465b27d998f4814b65d97de2cc5",
    "ae7f0400991647faf6d6d89c99db91b8",
    "61693d4bc28b9c4c13c25cba7b285047",
    "912a869cb1bf8fbf561b27ea204b5825",
    "a91ae32347e2c7881997c73191599e8e",
    "0ed6a7889593eddb05ca7a3fa67bcce8",
    "a9eb7ed838d55c1cb03f908d574b17f5",
    "f7f624d69c1b609dde421b1943eafeff",
    "8a986eba480a25ffd67f557b9c3fca67",
    "bf2f556926ba4892e6b5694a21cd9afe",
    "1eb846f4e762f12bea3c32dbadfaf41c",
    "c399a4a2157b4e4a09f3274a3363a553",
    "b76a67616d0666192c9005294f15e777",
    "d356f106c363dd5db29796516d57447b",
    "60ef85835bd95ba32ad9002593df724b",
    "544e6f384327b05bab1beff29dbea890",
    "f0ff805468583a71b9929a50228cc2db",
    "7b78c77d7d9d5f69c426908bdd3b3dbd",
    "d6021d8bf7ff5efed0b23b73ea2867af",
    "16082a6f243c20711ac4b07e52295be5",
    "a49bd4ee83ad3d55ff5621c496e6da6d",
    "6be644b51b587065650e194be3c90f67",
    "f55d21ce35afc24e585331c0cae00de1",
    "6ea1dcf0dae8a45561e618b9897dcff9",
    "60f4858f244aea4edcf376b76b8cb585",
    "11ad55c2aadb758b52b82f096e3c5dab",
    "3aeffcc6b0474dcb4845efdd91d029d2",
    "833dc71698e0d909c5b8413776fe5fa6",
    "34f5da0385b5da5a0d93755edbd30fde",
    "92c07e9ab0f04d553cc8cd5edcdec4f1",
    "15d708e684bd42ca3b4f67061799d49d",
    "9c1bc7e0f7e51547f3ccee5629bd8ca3",
    "f0dda502112f8104ff307dfd93bbc03c",
    "5506501ffb5fe92a4f6578af05a51f7f",
    "172710e1994f3e50b7ca340e7b84154e",
    "c93a6041f758a5a9ba60a1f3568a33f9",
    "e52e72a420cd3d676e3622092fca485c",
    "813a341e18f44b87841c9fd88ae20f71",
    "858b87cc3490bd58fdb3aa4ee3faf675",
    "e3e9eccf44421d12c43c5b8846e8c58d",
    "f9af7980a3d1e057b7fe7cafcbdb4e62",
    "55e6fc273fe0698a896ee4fcc897a44c",
    "ef80e1af045a549f56df22113c82233c",
    "d6e1aebe9ac7f530b46ed8e62769a501",
    "3cbc7269e3b90bcd76958b1bfa6baf93",
    "c08a05700a5245ea11b64f6eeb7dad40",
    "59f5994fae2d1101674e4fa1ec281087",
    "45930a48ca7b4960fd1ec0a2ff5be433",
    "bc9d77dff6dbfc4c484e766dcb403407",
    "70f9cee6336da43166fbe8393db6acb8",
    "1932dd5e0d77cd323dd8d4aa219bec52",
    "aa436df095ab310d67bdec078b5ab0f6",
    "d9b662a3b25ae607a49a69726823b176",
    "9c1ec16bef362c757936b00cde773572",
    "ffbcad9e10a575be0eadc440626e1fe2",
    "adc8e1c87659ac2568953eb8bb5ea3f2",
    "5cade2b758a71e8433c5480844b20dc8",
    "3c4a338aecb744edb90d7ac254c8977e",
    "0e81b0c0499de5097a1f399402234f4e",
    "43c0b9b16c7434970e6950db44110493",
    "a74646e4df212e1f95aec5cc35c51f9a",
    "485ea677839646d2375b3159603562bc",
    "ae8bcf9245305c8e30c0ae23e13d31e8",
    "68da2cd4a8dfbd79efecb54e70afc8d9",
    "a6b86d6ab022b92c9e5e6dd27db3f3d4",
    "5cdd514868f2c3e3c3260b14c40d747a",
    "c5645b985d551254e92753ebdbca5dd5",
    "1e81fccf0bd6f24b3659b781eb10b379",
    "93ad7593e5fa06ce4cf1d6e7f5976b00",
    "5a88f10a0b05a9fa81d94465db3fe1cc",
    "bf82703e3687ef17df21acbfd5f47bd3",
    "9c32a43a2435cc7d0e492131eccab7b3",
    "33a9bf1c326f2d8ffb10ab0654489ff3",
    "b49d4b93d10d414e506ac89fcb171019",
    "62aeba95156387d89d6e0a26ba9e9afb",
    "a899e75f2ce8e819404475e6b42bb845",
    "ca69f49acaa20ba7f07632c3f08419cc",
    "54578fa0d2c26b289a2b05ef4c264fc6",
    "167476ad6ec48aeb8be0673cdc8cc571",
    "fc868cf4072a4e7d25032f5cb026a01b",
    "185acc480e780041b0c264bf3d8e4295",
    "9c8ce934f354b7888e65a24f093fa580",
    "f860a5985bc2b229f147a06556731b07",
    "150e6cd31b9c3b097f720f96026c8233",
    "71ee32ae3d5d87519381b8b276e8cbc7",
    "2fd4a1bb5d3f1241ba77067957980065",
    "a8bc9c6f32abc123325e2ff1de054213",
    "40e5f5f30f8c369d734fab0a65e01dad",
    "ad2b89c9a7497b3a4acbd6dbd2a86f1f",
    "f59446d7ce66ed192a65d2c5752fffee",
    "25b18bd101dbd3c8042f172d8a7fe371",
    "89d3ea4946e8e745fadafec53540b405",
    "74b1f6aba6aa8c3249cd1699d2197354",
    "f134bd081a680695eb8e1cda75fc2538",
    "1f6db978767c795d76d0b25be03d9a6d",
    "dac4e4870011a8a60639ea466fe83964",
    "3be5cfba68aaf4383d2eded9c9fe4aa3",
    "099eabfe55446b8cdd7275eb76b8fd7f",
    "fb9a99feae8f5fb88c90e7ca0e0b4eab",
    "3737e627fb9e3806ec9ba9b22322296d",
    "85b875731503de33713446e4589352e0",
    "61246b682a2bb448ec6046c891fe0790",
    "29b87c2872d11dbf686e97f47179e76a",
    "cd0e271651c164df4ae21930770bc10c",
    "43fad8eaaa97cdce1ce10632cf3fd02d",
    "e4729fccbb2b79f1a3f2ebd83db112e2",
    "7cdf7597293e3f02ed15cebf87e187e3",
    "7648cf6b8b7db2fc8f4e4062704db126",
    "60b121d07250ea767599c50db40398f7",
    "36199201664015fbfeccb95cecc3dfb6",
    "3c7932e970bcb60a18b98347004d3133",
    "b346087f1a9a92f1d445b9094024d628",
    "0dba069ed1c7b37daf5a96313a0f1162",
    "3f8c6cf956a48fd5ee3b30afa579f673",
    "25b12d1c7d3892dc0958d5406116fed1",
    "6130070aa28897436095d4b0aaa6371f",
    "48d6eb1db877374cd61e10010cb189fc",
    "f326f1e5b9d8f98abcfe16bec627e042",
    "3ce7b0f55840ddb01b4ab8d0954a76aa",
    "38c9249f968a37b1c25491c96c190b9b",
    "fba3418f738087cc405daa12499ebff9",
    "ecd8765f18aa07a2c1a60f4f11072493",
    "aa9bdb1fd5e0a3bea02959a18918b4c3",
    "aa278cb79709ea39c32ea743dabafe2d",
    "52779a37550d907411d9fc48d24d1e70",
    "cee4ab47e458934a4ea9ae833e0a199b",
    "ff7e62680be484ac868a9c51a9711334",
    "23baca58f67d2c42cc4d20bd47d9163d",
    "9655db0e7bff8fdc953e33bfa686a5f9",
    "d33a5d0024b73aefc465d18fa173ad94",
    "c8d2a66e097f200611560d50bf6cd6ef",
    "7a621fdc589e41d8bad122b514c116aa",
    "942e9eb4427eb3305c450fa1508de00a",
    "ce8a2276a7641a90e80039848c2f34be",
    "b96142a3560967c53ea6c3019086175a",
    "0f6265f845fdd31cbf95121ad5f01f82",
    "b1a6f05da10af9f2eb141453d444f1af",
    "36239656cdc39ef315cefd2f8b3509a9",
    "cc6c79b70ca96311dc9d92002e19b4bb",
    "30210b596f3814d599f34b21d6cd2190",
    "e5f44c5837a896d8285889260202cf0c",
    "e4151f77f5a856cb3ff9127dc2070874",
    "de036614b08ca147cd22da0f7fc1fe13",
    "4d11a57bfc4704ebdeccb89bc97bc3fd",
    "17cac5791929799950161ec160c157f3",
    "4f092b40afded2507069665a0a640eb0",
    "ba70b0103d1beecb8ae0e7e4a0ab9425",
    "51ed428b07cc7fd040a0022df5914722",
    "0d9ef81136ebfdcec9939e3a4fc80192",
    "515c7f1197258b7cb8c4bc1602c50acc",
    "f3a926ba771bef84e80535659b386ec6",
    "924ea60ff4b8f78b80459539bb5fbc66",
    "5f1b52469db1d792be0681e18171a72b",
    "5962ce3016045ee3c0915f7921db6a12",
    "15c2ad42563b265c24b0060f31d89c60",
    "661bb489a0ed01cb56a79db63a1edb23",
    "1de130d6b1bef256c66adb1777f8e346",
    "a14c1ce02dca50befa8c48e691951ba0",
    "27aec2aa20352b3f4e7edaf4bf47ac98",
    "1134fc0c39d43ffcdcf4fe9890809017",
    "5d705061f1c6ac284ccdecc0b63e15fc",
    "dd1f5d2cc6ab710b19dea467b9760634",
    "e7ddfe67cbdfc58837c2b64ca3b32a67",
    "0c768df7dbae316623e705d051d3283e",
    "f2ca6701f26c9c67f104dc7905a36ad3",
    "a5bcf2bee12c2c527b0451a20849db75",
    "c35b200668f9b18e35c98c307eaf5124",
    "6d423ada865be792997eec7332cba253",
    "b3ec6b6a3d520f0f712c678c95618368",
    "17f33e6e3cc7d3b6402c4b802c3f8eae",
    "52acf12c38960340a65f24c143589f79",
    "80df11f7c8a47eceba855873c0b162fb",
    "88b67d319a1c7b7b64305b45c874345d",
    "d7f04aa44dbdcc4ce65792ef9a41f564",
    "4aabb8eb523415502cebb0ec150614cc",
    "c7201bb39c82c649de84840ffbd5d3b0",
    "976dafa9a228f275fde2261116e8a00e",
    "be53b9a30bca5653eed400be7aeafd85",
    "9b3439c161532db9a7b366ea23aaa129",
    "97d39a7222bc7dcdf1040705d1cdb0c5",
    "264924223b59720d8592873c86e843c0",
    "8f65eb8d9440f78fc1c3d88d7b6b0fb8",
    "84ae740efe9e56ab1b2d3760e4a4ef77",
    "ab5d80d32fefa9bd123f8a63c8cb65c2",
    "6dc3b8d6bd986f32fccba84c680aa98a",
    "28bf2709dd23eab44bff0ffcfec976b7",
    "0ebabea308db72d40ad95c81c9b0f6f1",
    "a173e60bf9c01f448aa275e178432295",
    "880e99942ad3a924f883bb7f6bb9081d",
    "cf62379fa9e553c1aa6ed991d4175858",
    "9259f623b4138bc03c85e6a68f776f05",
    "a3fbffe5aa69aef8c6c0794d92243399",
    "f50bde7a378f2d1949ab7b5133c24a8f",
    "4534a3b501caa12cae8fd4921db3bb7c",
    "fe06ae8b6a4023d91a01d04eb821cd76",
    "79de85bc4f4e5169a7a4c031a2bf5566",
    "b7488097640a8cfd2da50fc2f1d906ed",
    "c3aa94ae14a6a27eaea668f9ee0d790c",
    "0929eee2dbfae6e6aba69e85d52b7876",
    "b85d295608efeea7f9b0ff8df3106347",
    "8d9f4461d894616d748a4a0a4eda37d4",
    "ca5a18a59d2602125cde6e05c83fb19c",
    "0572763010cae8e924f93721cc9e46d6",
    "e676ea05bd5fd69b3461f1ffa46f9e70",
    "fed5bdf5115415b3140609fcfe3390b2",
    "9546cdd4f15dcea54ab01a636ef319d6",
    "ba8d3dc108687b5600b3f2015db3c3d7",
    "fe68c0a0c1703c4d184edc69d4b282c8",
    "21be93b0fe22fc4aaf20b0478dab4904",
    "47fb77a24d7c36990f59b1c0b4e6950c",
    "64cac8d10c88b21dc393180be5090afb",
    "2398899952ecfb38deb3b16d8b119593",
    "0d3a83c71952b9d49fab0001dfbab4aa",
    "d16dbc15e23f49fc8b4e590fa72648e1",
    "bec0144a98bc2279713b00888e2936e3",
    "e930a0f6872471b0fc598185e948e089",
    "572ac21dd614965ca15790cd1143ba32",
    "8414d56a4d4c10004f0133bd4314bf6d",
    "3e50f05c10793aa8d5b66d2494dacab3",
    "7cba509721c5514b2ee3d36cc7e8ca35",
    "0186633e1f32230dbd46bee616b5de97",
    "e2fb8bc5524f9baaa1ab591652288495",
    "658548bdbce654f0aee5070018ee4c5c",
    "956028485694d01310174dc9636a1c86",
    "343f038e3e288ae1c405873207f85d9e",
    "97750c6e3963068702ae4b7b585c31e0",
    "7fcf5f2e5217e32d3ddceb8867ab2ab7",
    "685c1ca85c63d175c21b0275749daf5f",
    "d34642a7f5cb1ffbed2f25c34ea45eb8",
    "a8b47f1162f9d8ae6a72d4bfb6432f2f",
    "160b3acabed92ddc1999eb42d349b98d",
    "b2a948005f83d89ec577dd1ec8fe17fb",
    "c7cc94a89f8b2a0553c9cdf5ed35c55a",
    "a1fa5f5315aee0e5de784856ec8d0743",
    "9c05840f6992f9f35bf64ee98c7c16ad",
    "5da935276ac58205eb86e1fa4600f35b",
    "95d9dfcf693ad284c038f67c51496762",
    "808a1c173f7afab14f8d9fb6f78f6e6a",
    "9d765f3d2729e5f2db3597839c1fa344",
    "e389c34261258002e39b73176702746b",
    "0a30fc8db9c603cae892742a761541ec",
    "ceb8459d4a91958daffbf56a4bcee5df",
    "5ce306ee000722d7c26d802a79877c3b",
    "a32bfe04de1b0e07e9f3ff6403845ec7",
    "6bd76aa0fb1986c241b8fea0f4f2ca18",
    "292e37fb4da91ea2c852d01a5a9283e8",
    "dab3c5923819851de3a2704f9a7b5e00",
    "bad527f0f1115438b6a2d8b99b75b0c1",
    "aa6bbb3ac00c41367e93f2969a492ee7",
    "f0fb3176141bb818434e26c5454807c4",
    "0333d4046056532523662da757322705",
    "b7a9a84ac87056284ace0a0f54b51617",
    "d5a85d148de08696863b33c7941f86f9",
    "2ce9b5a7dbe3b0170fb8a392893f8383",
    "ce9b3be01086926a3a6e83308155c3cd",
    "32fa6d2464e5d06e65ca5387bb562bb3",
    "6e3e945da1472e1274b0fab1ad1acef0",
    "6a342ab1569846a9212eab9ec5eec08e",
    "b38f92b78615f584153427e8e581005c",
    "03543b04e128a51a3fcc8ffabccf4a74",
    "5b9faa56d1a59b0b5e12b45d60302ebe",
    "22cf3d9bc288bf9ae954731f114e8bd1",
    "47ac6de23811faddbcde198ad41b3431",
    "f7ea70abf72cf7ae611ed1bf1271e8f6",
    "fff19f453ad7fbe308d712f078b258ea",
    "0ec50c55e94dae5a220a9b6cd2430ed9",
    "71884fb0c8a9fbd118e03d9f442b9a93",
    "0c95352ae4e40ec464637765667376c0",
    "b8d997bc96caa49a02d44d5e3cb68f81",
    "ff315ea229023ea498b836cddb320b89",
    "2c2df97ade28a35e4d04ab3157ce779a",
    "360be737dc97b396d3e3f2c53011d898",
    "680cfd95f4352e29719a8b5694b1c639",
    "471c312a297f9101c3daec1c971ba5a2",
    "cd0e3ac5cdc82a9cfb77436b9b7c8f86",
    "7ef39063b1c3a112faaa4a5025ebdf13",
    "fe2b298d0011387635532b5239f74bbc",
    "5b6eaa9adfc8f2666116d05efed4e860",
    "0d66697afa2a8da51eba022ab65f5a0a",
    "dde9ddd3b16453f016fb09e483a64a51",
    "3b8479e83a82c390daad15ab76b93f2f",
    "e811f2f67816198b62798da20b4da6ff",
    "bba1ada2ecbf4e3a5c68e157ee8e658e",
    "55309f40d7c79d384bc597d56b7f0f4c",
    "563f743b519f084a8160da996553e7dc",
    "b5cdb60573799e559c153e2b2858a3ab",
    "870f56f3b74d96bf61bb192e0474c9cc",
    "43628820090b44c6751eab675bfd1533",
    "e377cd598333ebd2b20c177536626c93",
    "276b9f0ecea1ca3e8453626f63264673",
    "70681fd55d99784982d9c694f507e789",
    "85d3e5f4c1cd24428305ea8264d62c71",
    "66d5bcf12eb47cf060e0ea0aacbd23e4",
    "9fde0cff2cffecc447669bf070d34aa6",
    "4d83a5b037e7ee378437a358bbd2746d",
    "709a207e1bde320bdb271f4a33a7b1d8",
    "24c3cb0f319efac8a7a014adf7e410cc",
    "c92eab7076e2dbb87c59293511fba0d9",
    "cc8e44b23450163cf84e746340c52ffe",
    "c28f828bd365371beebdb32a1a414f5f",
    "7e53237989979daa9533046300a01998",
    "b4ae022cd07becec7aa080496cfb7b90",
    "a1f783e10087170158dbc10f0b2952d1",
    "8516013d799414d78ff86d19aa03846d",
    "48253939ac1237d81d24649662c1866b",
    "8e2a75435411de2355bacf5de479f481",
    "a2435d7dc9d4e5f9ec5e05e7e322dae1",
    "e8c935fb30368a55d6c6a7d38ad9a128",
    "e39fec846e3462429073c4822224e22d",
    "c48ac398d84723786627ba1c3e43110c",
    "c9a7a83c5564d7c0124ecbc42ad66c58",
    "9bb01772ae0e42e60de7da0c33928264",
    "6572680147896d3e994489b31b781735",
    "9acf6aa76f6e00f2eebdea783342c974",
    "b1aa3d2df8da67a7accc37902ba736db",
    "001fb250abbf4f1c4ec18a6dbaed379e",
    "d641d333bf3990800156995ba415ebe0",
    "3c9a07d13485af807c66e35c0a9fdb97",
    "a81aa796ccc1b9b1f2f71a5bf28c3576",
    "4f9c385f0788909352d0a265c0175b22",
    "adfca794c87dbb5f55b3d28042edc9a4",
    "0d783485282875ef0e2664d67e57c2e5",
    "2ba2b5e6d601f4252ae479a95f539212",
];

pub struct LocalServer {
    handle: axum_server::Handle,
}

impl LocalServer {
    pub fn stop(self) {
        self.handle.graceful_shutdown(Some(std::time::Duration::from_secs(2)));
    }
}

fn rsa_key_pem() -> Option<String> {
    let out = std::process::Command::new("openssl")
        .args(["genrsa", "2048"])
        .output()
        .ok()?;
    if !out.status.success() { return None; }
    String::from_utf8(out.stdout).ok()
}

fn generate_tls_certs() -> Result<(String, String, String), String> {
    use rcgen::{Certificate, CertificateParams, DnType, IsCa, BasicConstraints, SanType, KeyPair,
                PKCS_RSA_SHA256, PKCS_ECDSA_P256_SHA256};

    // Prefer RSA 2048 so the game's TLS 1.2 libcurl can use ECDHE-RSA cipher suites.
    // Fall back to ECDSA P-256 if openssl is not on PATH (e.g. bare Windows).
    let (ca_alg, server_alg, ca_key_opt, server_key_opt) = match rsa_key_pem() {
        Some(ca_pem) => match rsa_key_pem() {
            Some(sv_pem) => {
                let ca_kp = KeyPair::from_pem(&ca_pem).map_err(|e| format!("CA key: {e}"))?;
                let sv_kp = KeyPair::from_pem(&sv_pem).map_err(|e| format!("Server key: {e}"))?;
                (&PKCS_RSA_SHA256, &PKCS_RSA_SHA256, Some(ca_kp), Some(sv_kp))
            }
            None => (&PKCS_ECDSA_P256_SHA256, &PKCS_ECDSA_P256_SHA256, None, None),
        },
        None => (&PKCS_ECDSA_P256_SHA256, &PKCS_ECDSA_P256_SHA256, None, None),
    };

    let mut ca_params = CertificateParams::default();
    ca_params.alg = ca_alg;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.distinguished_name.push(DnType::CommonName, "Evolve Revival CA");
    if let Some(kp) = ca_key_opt { ca_params.key_pair = Some(kp); }
    let ca_cert = Certificate::from_params(ca_params)
        .map_err(|e| format!("CA cert: {e}"))?;
    let ca_pem = ca_cert.serialize_pem()
        .map_err(|e| format!("CA pem: {e}"))?;

    let mut server_params = CertificateParams::default();
    server_params.alg = server_alg;
    // *.my.2k.com covers api.my.2k.com and other single-level subdomains.
    // The Pinenut cached doorman config points services at a 4-label subdomain
    // (508223012e5a5ff19f30a391b2bdadc0.my.2k.com) which wildcards can't cover —
    // add it explicitly so kando's TLS validation passes for those calls too.
    server_params.subject_alt_names = vec![
        SanType::DnsName("*.my.2k.com".to_string()),
        SanType::DnsName("508223012e5a5ff19f30a391b2bdadc0.my.2k.com".to_string()),
    ];
    server_params.distinguished_name.push(DnType::CommonName, "*.my.2k.com");
    if let Some(kp) = server_key_opt { server_params.key_pair = Some(kp); }
    let server_cert = Certificate::from_params(server_params)
        .map_err(|e| format!("Server cert: {e}"))?;
    let server_cert_pem = server_cert.serialize_pem_with_signer(&ca_cert)
        .map_err(|e| format!("Server cert pem: {e}"))?;
    let server_key_pem = server_cert.serialize_private_key_pem();

    Ok((ca_pem, server_cert_pem, server_key_pem))
}

fn patch_ca_bundle(game_root: &std::path::Path, ca_pem: &str) -> Result<(), String> {
    let bundle_path = game_root.join("ca-bundle.crt");
    let existing = std::fs::read_to_string(&bundle_path).unwrap_or_default();
    // Keep only the first cert (Pinenut's original CA), then append ours
    let first_end = existing.find("-----END CERTIFICATE-----")
        .map(|i| i + "-----END CERTIFICATE-----".len())
        .unwrap_or(0);
    let new_bundle = format!("{}\n{ca_pem}", &existing[..first_end]);
    std::fs::write(&bundle_path, new_bundle)
        .map_err(|e| format!("Write ca-bundle.crt: {e}"))
}

pub async fn start(game_root: &std::path::Path) -> Result<LocalServer, String> {
    let (ca_pem, cert_pem, key_pem) = generate_tls_certs()?;
    patch_ca_bundle(game_root, &ca_pem)?;

    let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem(
        cert_pem.into_bytes(),
        key_pem.into_bytes(),
    )
    .await
    .map_err(|e| format!("TLS config: {e}"))?;

    // Bind on 4430 — an iptables OUTPUT rule redirects 127.0.0.1:443 → 4430 so the
    // game's kando client (which always dials port 443) still reaches us without
    // requiring the binary to hold CAP_NET_BIND_SERVICE or run as root.
    let addr = "127.0.0.1:4430".parse::<std::net::SocketAddr>().unwrap();

    // Bind synchronously first — fail fast if port is in use, no zombie listeners
    let std_listener = std::net::TcpListener::bind(addr)
        .map_err(|e| format!("Failed to bind {addr}: {e}"))?;
    std_listener.set_nonblocking(true)
        .map_err(|e| format!("nonblocking: {e}"))?;

    let handle = axum_server::Handle::new();
    let app = build_router();
    let handle_clone = handle.clone();
    tokio::spawn(async move {
        axum_server::from_tcp_rustls(std_listener, tls_config)
            .handle(handle_clone)
            .serve(app.into_make_service())
            .await
            .ok();
    });

    Ok(LocalServer { handle })

}

fn inst(host: &str, base_uri: &str, actions: &[&str]) -> Value {
    json!({
        "protocol": "https",
        "host": host,
        "port": 443,
        "baseUri": base_uri,
        "actions": actions,
        "isProduction": true,
    })
}

fn kando_ok(result: Value) -> Json<Value> {
    Json(json!({
        "result": result,
        "header": {
            "code": 0,
            "cache": {
                "onlineTtl": 86400,
                "offlineTtl": -1,
            }
        }
    }))
}

async fn log_request(req: Request<Body>, next: Next) -> Response {
    use std::io::Write;
    let method = req.method().clone();
    let uri = req.uri().clone();

    // Buffer the body so we can log it AND still pass it to the handler
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, 4 * 1024 * 1024).await.unwrap_or_default();

    // Log all request bodies to diagnose kando protocol
    if !bytes.is_empty() {
        let _ = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open("/home/navitank/Desktop/EvolveFilesLegacy/request_body.log")
            .and_then(|mut f| {
                writeln!(f, "=== {} {} ===\n{}\n", method, uri, String::from_utf8_lossy(&bytes))
            });
    }

    // Reconstruct request with buffered body
    let req = Request::from_parts(parts, Body::from(bytes));
    let resp = next.run(req).await;

    let _ = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("/tmp/evolve_server.log")
        .and_then(|mut f| {
            writeln!(f, "{method} {uri} -> {}", resp.status())
        });
    resp
}

fn build_router() -> Router {
    Router::new()
        // Doorman — game uses POST /doorman/1 (not GET /doorman/1/configs/generate)
        .route("/doorman/1", post(doorman_post))
        .route("/doorman/1/configs/generate", get(doorman_config))
        // Auth — /{steam_id}/auth/two_k and /{steam_id}/profile/...
        .route("/{player_id}/auth/two_k", post(two_k_auth))
        .route("/{player_id}/profile/get_by_platform_account_id", post(profile_get))
        // Build config — versioned path the game actually calls
        .route("/PC/Production/{version}/{file}", get(build_config))
        .route("/singlesignon/1", post(sso_logon))
        .route("/sso/1/logon/{game}", post(sso_logon))
        .route("/apps/1/find", post(apps_find))
        .route("/sessions/1/create", post(session_create))
        .route("/sessions/1/heartbeat", post(heartbeat))
        .route("/entitlements/1/appOwnership/{group}", get(app_ownership).post(app_ownership))
        .route("/entitlements/1/checkAppOwnership/{group}", get(app_ownership).post(app_ownership))
        .route(
            "/entitlements/1/firstPartyMapping/{platform}/{platform_id}",
            get(entitlements_all).post(entitlements_all),
        )
        .route("/entitlements/1/mapping/{group}", get(entitlements_all).post(entitlements_all))
        .route("/entitlements/1/grants/find", post(grants_find))
        .route("/status", get(server_status))
        .route("/build_config", get(build_config))
        .route("/heartbeat", post(heartbeat))
        .route("/stats/1/configs", get(stats_configs).post(stats_configs))
        .route("/grants/1/find", post(grants_find))
        .route("/queue/waittime", get(queue_waittime).post(queue_waittime))
        .fallback(stub_ok)
        .layer(axum::middleware::from_fn(log_request))
}

async fn doorman_post(_body: axum::body::Bytes) -> Json<Value> {
    // body is logged by middleware to request_body.log
    doorman_config().await
}

async fn doorman_config() -> Json<Value> {
    // Doorman is NOT wrapped in kando_ok — kando reads services/clientConfigSettings from top-level
    Json(json!({
        "accessToken": Uuid::new_v4().to_string(),
        "expiresIn": 86400,
        "refreshToken": Uuid::new_v4().to_string(),
        "refreshExpiresIn": 86400,
        "tokenType": "Bearer",
        "playerId": null,
        "sessionId": Uuid::new_v4().to_string(),
        "platformType": 1,
        "onlineServicePlatform": 1,
        "services": [
            {"serviceName": "doorman",      "serviceInstances": [inst("api.my.2k.com", "doorman/1",      &[])]},
            {"serviceName": "content",      "serviceInstances": [inst("api.my.2k.com", "content/1",      &[])]},
            {"serviceName": "singlesignon", "serviceInstances": [inst("api.my.2k.com", "sso/1",          &[])]},
            {"serviceName": "sessions",     "serviceInstances": [inst("api.my.2k.com", "sessions/1",     &[])]},
            {"serviceName": "apps",         "serviceInstances": [inst("api.my.2k.com", "apps/1",         &[])]},
            {"serviceName": "entitlements", "serviceInstances": [inst("api.my.2k.com", "entitlements/1", &[])]},
            {"serviceName": "players",      "serviceInstances": [inst("api.my.2k.com", "players/1",      &[])]},
            {"serviceName": "storage",      "serviceInstances": [inst("api.my.2k.com", "storage/1",      &[])]},
            {"serviceName": "storefront",   "serviceInstances": [inst("api.my.2k.com", "storefront/1",   &[])]},
            {"serviceName": "telemetry",    "serviceInstances": [inst("api.my.2k.com", "telemetry/1",    &[])]},
            {"serviceName": "stats",        "serviceInstances": [inst("api.my.2k.com", "stats/1",        &[])]},
            {"serviceName": "news",         "serviceInstances": [inst("api.my.2k.com", "news/1",         &[])]},
        ],
        "clientConfigSettings": {
            "doormanConnectTimeout": 5000,
            "doormanRequestTimeout": 10000,
            "defaultRequestTimeout": 10000,
            "logLevelMin": 0,
            "logLevelMax": 5,
            "logLevel": 4,
            "debugModeMin": false,
            "debugModeMax": false,
            "debugMode": true,
            "restLogToFileModeMin": false,
            "restLogToFileModeMax": false,
            "restLogToFileMode": true,
            "restLogToFilePath": "rest.log",
            "autoCacheLogin": false,
            "minRequestTimeout": 3000,
            "assertTelemetry": false,
            "appHasStorefront": false,
            "storefrontHasConsumables": false,
        }
    }))
}

// The DLL (EvolveLegacyRebornServer.dll) sends a 2K-internal UUID as `2k_player_id`
// in the binary protobuf auth body (e.g. "2357d522a223d8d57b05071505274b6b"). It
// also hardcodes that same UUID in its embedded checkAppOwnership/grants responses,
// which it serves internally without hitting our server. If we return a different
// playerId (like the Steam ID URL param), kando's session state won't match the
// DLL's hardcoded entitlement data and my2K fails. Extract the UUID from the body.
fn extract_2k_uuid(body: &[u8], field_name: &[u8]) -> Option<String> {
    let pos = body.windows(field_name.len()).position(|w| w == field_name)?;
    let is_hex = |c: u8| matches!(c, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F');
    // Search the 128 bytes before the field name for a 32-char hex UUID.
    // Scan from the end (closest to the field name) to avoid matching an earlier
    // field like 2k_platform_id when we want 2k_player_id.
    let lo = pos.saturating_sub(128);
    let region = &body[lo..pos];
    for i in (0..region.len()).rev() {
        if i + 32 <= region.len() && region[i..i+32].iter().all(|&c| is_hex(c)) {
            let before_ok = i == 0 || !is_hex(region[i - 1]);
            let after_ok = i + 32 == region.len() || !is_hex(region[i + 32]);
            if before_ok && after_ok {
                return String::from_utf8(region[i..i+32].to_vec()).ok();
            }
        }
    }
    None
}

async fn two_k_auth(Path(steam_id): Path<String>, body: axum::body::Bytes) -> Json<Value> {
    let two_k_id = extract_2k_uuid(&body, b"2k_player_id")
        .unwrap_or_else(|| steam_id.clone());
    kando_ok(json!({
        "accessToken": Uuid::new_v4().to_string(),
        "expiresIn": 86400,
        "tokenType": "Bearer",
        "playerId": two_k_id,
        "sessionId": Uuid::new_v4().to_string(),
        "refreshToken": Uuid::new_v4().to_string(),
        "refreshExpiresIn": 86400,
        "isNewPlayer": false,
        "hasPlayedApp": true,
        "dobNeeded": false,
    }))
}

async fn profile_get(Path(steam_id): Path<String>, body: axum::body::Bytes) -> Json<Value> {
    let two_k_id = extract_2k_uuid(&body, b"guid")
        .or_else(|| extract_2k_uuid(&body, b"2k_player_id"))
        .unwrap_or_else(|| steam_id.clone());
    kando_ok(json!({
        "player": {
            "publicId": two_k_id.clone(),
            "platformAccountId": steam_id,
            "alias": "Player",
            "isNewPlayer": false,
            "hasPlayedApp": true,
        }
    }))
}

async fn sso_logon() -> Json<Value> {
    kando_ok(json!({
        "isNewPlayer": false,
        "hasPlayedApp": true,
        "accessToken": Uuid::new_v4().to_string(),
        "expiresIn": 86400,
        "tokenType": "Bearer",
        "playerId": Uuid::new_v4().to_string(),
        "sessionId": Uuid::new_v4().to_string(),
        "refreshToken": Uuid::new_v4().to_string(),
        "refreshExpiresIn": 86400,
        "dobNeeded": false,
    }))
}

async fn app_ownership(Path(_group): Path<String>) -> Json<Value> {
    kando_ok(json!({"ownsApp": true}))
}

async fn entitlements_all() -> Json<Value> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let player_id = Uuid::new_v4().to_string();
    let items: Vec<Value> = ENTITLEMENT_IDS
        .iter()
        .map(|id| {
            json!({
                "createdOn": now,
                "entitlementDefId": id,
                "isServerAuthoritative": true,
                "isValid": true,
                "ruleData": {"grant": true},
                "entitlementId": Uuid::new_v4().to_string(),
                "appGroupId": APP_GROUP_ID,
                "playerPublicId": player_id,
                "isAvailable": true,
                "isShared": false,
            })
        })
        .collect();
    kando_ok(json!({"entitlements": items}))
}

async fn server_status() -> Json<Value> {
    Json(json!({"status": "ok", "version": "env3-kando-envelope"}))
}

async fn build_config() -> Json<Value> {
    Json(json!({"version": "1.0.0", "build": 1}))
}

async fn heartbeat() -> Json<Value> {
    kando_ok(json!({"ok": true}))
}

async fn stats_configs() -> Json<Value> {
    kando_ok(json!({"statGroups": []}))
}

async fn grants_find() -> Json<Value> {
    kando_ok(json!({"grants": []}))
}

async fn queue_waittime() -> Json<Value> {
    kando_ok(json!({"waitTimeSeconds": 0}))
}

async fn stub_ok() -> Json<Value> {
    kando_ok(json!({}))
}

async fn apps_find(_body: axum::body::Bytes) -> Json<Value> {
    kando_ok(json!({"apps": [{"appId": APP_GROUP_ID, "owned": true}]}))
}

async fn session_create(_body: axum::body::Bytes) -> Json<Value> {
    kando_ok(json!({
        "sessionId": Uuid::new_v4().to_string(),
        "expiresIn": 86400,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_starts_and_listens() {
        let game_root = std::path::Path::new("/home/navitank/Desktop/EvolveFilesLegacy");
        match start(game_root).await {
            Ok(s) => {
                match tokio::net::TcpStream::connect("127.0.0.1:4430").await {
                    Ok(_) => println!("SUCCESS: server is listening on 4430"),
                    Err(e) => panic!("Server did not listen on 4430: {e}"),
                }
                s.stop();
            }
            Err(e) => panic!("start() failed: {e}"),
        }
    }
}
