package model

type EntitlementItem struct {
	CreatedOn             int         `json:"createdOn"`
	EntitlementDefId      string      `json:"entitlementDefId"`
	IsServerAuthoritative bool        `json:"isServerAuthoritative"`
	IsValid               bool        `json:"isValid"`
	RuleData              RuleData    `json:"ruleData"`
	EntitlementId         string      `json:"entitlementId"`
	AppPublicId           interface{} `json:"appPublicId"`
	AppGroupId            string      `json:"appGroupId"`
	PlayerPublicId        string      `json:"playerPublicId"`
	IsAvailable           bool        `json:"isAvailable"`
	IsShared              bool        `json:"isShared"`
}

type RuleData struct {
	Grant bool `json:"grant"`
}

const AppGroupId = "c3dc178f670ee769fe59e244610d66e2"

// EntitlementIds is the full list of 468 entitlement IDs extracted from Pinenut.
// All are granted to every player on logon (Pinenut behaviour).
var EntitlementIds = []string{
	"c7d22439bc13e53554776bbec4c175db", "bada12bfd30bcd02a609f7c88ca0a244",
	"d732cbb899e12be62d1fdfb6d43ac15c", "c0e9412ae0e437935f4fc1ef9db8add7",
	"47b1ec72d6dee517cc7fd896e5c66c93", "79c0b7fbb8b5a610d5665fe66e3c6c6a",
	"2ebd108f2cdce9e5315295786859eabe", "7ea5435a8af71001b790a6766d0b14b5",
	"5511deee62056a54d1a940fc8d14d1d7", "bb9e6744eb44a96c25987283f2150844",
	"034463e6f628e94147c366850693f3af", "ae60dc2f1dc7ce5e837e23d765308a7f",
	"09dc8569d41d18169dba133b646a0c7c", "ae16a01bd0f92671190a36f7edcd218c",
	"f26c3f0e0d4c89f9583403d08c68fb15", "669261b069a12f196ca6e083c5944059",
	"92f875f36cf4fe94265d07ac1574678b", "cb4b79d5d1e10b0b0a3dc98f044c0f14",
	"4b3b9650cf729c8ebd90354d22121f5e", "c5bed6ded5991a7d5deb29df9e270856",
	"43745436a426991ef359ac767af529bf", "5e83883ab63364d58502733b10186c31",
	"c816eb19a5a20bbc035fed633a4c04ca", "cf6ff1e376dec1a317459794d60d916a",
	"5d8e4486551dcd54b27224a159e8885d", "9c84c670e462589c20e5cf95afea516a",
	"7ad74999061ae851075957b3b65d3f66", "a96e8e5e664d6d6fd230731d59d62e94",
	"e946877ca525e8379d9929de9c4e5890", "f3deb191029a5baee0e980b8d3e02894",
	// TODO: populate remaining 438 IDs by extracting from EvolveLegacyRebornServer.dll:
	//   strings EvolveLegacyRebornServer.dll | grep -oE '[0-9a-f]{32}' | sort -u
}

// EntitlementDefIds maps parallel to EntitlementIds. Pinenut uses the same value for both.
var EntitlementDefIds = EntitlementIds
