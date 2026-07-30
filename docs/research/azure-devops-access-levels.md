# Azure DevOps `accessLevel` 官方資料研究

查核日期：2026-07-31

本文件只採用 Microsoft Learn、Microsoft 官方 REST API 規格，以及 Microsoft／Azure 官方 GitHub repository。本文區分三種容易混淆的概念：

1. REST schema 列出的列舉值。
2. Azure DevOps 目前公開支援、可對應到產品介面的存取層級。
3. 可以安全提供給 `adoctl user set-access` 使用者的設定值。

**結論：REST schema 列出某個列舉值，不代表它就是現行、公開支援且適合讓管理者直接指定的存取層級。**

* * *

## 核心結論

- 現行 Azure DevOps Services 文件列出的主要存取層級是 Stakeholder、Basic、Basic + Test Plans、Visual Studio Subscriber 與 GitHub Enterprise；其中 Microsoft 明確表示「多數使用者」適合 Basic。[Microsoft：About access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#supported-access-levels)
- 對組織直接計費的三種公開映射為：
  - Stakeholder：`accountLicenseType=stakeholder`、`licensingSource=account`
  - Basic：`accountLicenseType=express`、`licensingSource=account`
  - Basic + Test Plans：`accountLicenseType=advanced`、`licensingSource=account`

  此映射由 Microsoft 的「Programmatic mapping of access levels」表格明確定義。[Microsoft：Programmatic mapping of access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#programmatic-mapping-of-access-levels)
- Visual Studio 與 GitHub Enterprise 權益不是額外的 `AccountLicenseType`。它們需要 `licensingSource` 搭配對應的 `msdnLicenseType` 或 `gitHubLicenseType`；Azure DevOps 也會驗證使用者是否真的具備相應訂閱或授權。[Microsoft：Programmatic mapping of access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#programmatic-mapping-of-access-levels) [Microsoft：User and permissions management FAQs](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/faq-user-and-permissions-management?view=azure-devops#visual-studio-subscriptions)
- `earlyAdopter` 雖然仍存在於 REST schema 及 Azure CLI 的接受值中，但 Microsoft 明確說它只供 Microsoft 內部使用，**不應成為一般使用者可選的公開選項**。[Microsoft：About access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#programmatic-mapping-of-access-levels) [Microsoft：az devops user](https://learn.microsoft.com/en-us/cli/azure/devops/user?view=azure-cli-latest#az-devops-user-update)
- `professional` 仍存在於 REST schema，Azure CLI 也接受它；但現行官方存取層級映射沒有為它定義產品介面名稱、權益或計費語意。**查無足夠官方資料可把它說明為某個現行存取層級，因此不應標成常用，也不宜在沒有警告的情況下公開。**[Microsoft REST 7.2-preview schema：AccessLevel](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1431-L1473) [Microsoft：az devops user](https://learn.microsoft.com/en-us/cli/azure/devops/user?view=azure-cli-latest#az-devops-user-update)

* * *

## `AccessLevel` 欄位

官方 7.2-preview schema 的 `AccessLevel` 物件包含下列欄位。[Microsoft REST 7.2-preview schema：AccessLevel](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1431-L1667)

| 欄位 | 用途 | 設定時的判斷 |
| --- | --- | --- |
| `accountLicenseType` | 組織帳戶提供的授權類型 | 只有搭配 `licensingSource=account` 才符合官方 request 規則 |
| `licensingSource` | 授權權益來源 | 決定要解讀 `accountLicenseType`、`msdnLicenseType` 或 `gitHubLicenseType` |
| `msdnLicenseType` | Visual Studio／MSDN 訂閱類型 | 只有搭配 `licensingSource=msdn` 才符合官方 request 規則 |
| `gitHubLicenseType` | GitHub 授權類型 | REST 7.2-preview schema 要求搭配 `licensingSource=gitHub` |
| `assignmentSource` | 直接或群組規則等指派來源 | 應視為服務端產生的結果，不是 `set-access` 的使用者輸入 |
| `licenseDisplayName` | Azure DevOps 顯示的授權名稱 | 應視為服務端衍生的顯示欄位 |
| `status` | 使用者在組織中的授權狀態 | 應視為查詢結果；官方變更 access level 範例未寫入此欄位 |
| `statusMessage` | 狀態補充訊息 | 應視為查詢結果 |

官方單一使用者更新 API 使用 JSON Patch，範例以 `replace /accessLevel` 寫入 `accountLicenseType=express` 與 `licensingSource=account`，需要 `vso.memberentitlementmanagement_write` scope。[Microsoft：Update User Entitlement](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/update-user-entitlement?view=azure-devops-rest-7.1)

**官方更新範例沒有把 `assignmentSource`、`licenseDisplayName`、`status` 或 `statusMessage` 當成管理者輸入；CLI 應只寫入有公開組合規則的欄位。**

* * *

## `AccountLicenseType` 全部列舉值

REST 7.1 與 Microsoft 最新公開的 7.2-preview 規格皆列出 `none`、`earlyAdopter`、`express`、`professional`、`advanced`、`stakeholder` 六個值。[Microsoft Learn：REST 7.1 AccountLicenseType](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#accountlicensetype) [Microsoft REST 7.2-preview schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1435-L1473)

下表的「使用情境」以官方公開映射為準；官方未定義者不自行猜測。

「常用」標示是依官方建議與適用情境整理，不代表 Microsoft 公布的實際使用率；只有 Basic 有「多數使用者適用」的官方明文。

| REST 值 | 公開語意與對應 | 可否作為一般 `set-access` 選項 | 文件標示建議 |
| --- | --- | --- | --- |
| `stakeholder` | Stakeholder；免費、功能受限，適合只需工作項目、討論與儀表板等功能的人員。[Microsoft：Supported access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#supported-access-levels) | 可以；搭配 `licensingSource=account` | **常用** |
| `express` | Basic；提供大多數功能，Microsoft 表示多數使用者適合 Basic。[Microsoft：Supported access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#supported-access-levels) | 可以；搭配 `licensingSource=account`。CLI 可把較易懂的 `basic` 映射為此值 | **最常用** |
| `advanced` | Basic + Test Plans；包含 Basic 與 Azure Test Plans 完整功能。[Microsoft：Supported access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#supported-access-levels) | 可以；搭配 `licensingSource=account` | **常用於測試與 QA 角色** |
| `none` | 「沒有 account 授權類型」的 sentinel。官方 Visual Studio 與 GitHub Enterprise 映射會使用 `none`，再由其他授權型別欄位表達權益；它本身不是一個可供人員使用的存取層級。[Microsoft：Programmatic mapping](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#programmatic-mapping-of-access-levels) | 不應單獨搭配 `licensingSource=account` 暴露為一般選項 | 內部組合值 |
| `earlyAdopter` | Microsoft 明確標示為只供 Microsoft 內部使用。[Microsoft：Programmatic mapping](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#programmatic-mapping-of-access-levels) | Azure CLI 目前接受此字串，但產品文件禁止把它當公開存取層級使用。[Microsoft：az devops user update](https://learn.microsoft.com/en-us/cli/azure/devops/user?view=azure-cli-latest#az-devops-user-update) | **禁止一般使用** |
| `professional` | schema 與 Azure CLI 接受值仍保留此字串，但現行官方映射沒有定義其產品名稱、功能、計費或建議用途。[Microsoft REST 7.2-preview schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1435-L1473) [Microsoft：az devops user update](https://learn.microsoft.com/en-us/cli/azure/devops/user?view=azure-cli-latest#az-devops-user-update) | 技術上是 Azure CLI 接受值；但查無足夠官方資料證明它是現行公開支援的管理選項 | 舊版／未文件化；不標常用 |

Microsoft 官方 Terraform provider 同樣接受六個 schema 值，並額外接受 `basic` 作為 `express` 的別名；其預設值為 `express`。[Microsoft Terraform provider：user_entitlement](https://github.com/microsoft/terraform-provider-azuredevops/blob/b00216db479c11cae8e7cfec42a79c29af853898/website/docs/r/user_entitlement.html.markdown#L20-L32)

### 常用的直接指派範例

```json
{
  "accountLicenseType": "stakeholder",
  "licensingSource": "account"
}
```

```json
{
  "accountLicenseType": "express",
  "licensingSource": "account"
}
```

```json
{
  "accountLicenseType": "advanced",
  "licensingSource": "account"
}
```

這三種組合與官方公開映射完全一致。[Microsoft：Programmatic mapping](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#programmatic-mapping-of-access-levels)

* * *

## `LicensingSource` 全部列舉值

REST 7.1 列出 `none`、`account`、`msdn`、`profile`、`auto`、`trial`；官方 REST 7.2-preview schema 另加入 `gitHub`。[Microsoft Learn：REST 7.1 LicensingSource](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#licensingsource) [Microsoft REST 7.2-preview schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1524-L1567)

| REST 值 | 官方可驗證語意 | 設定建議 |
| --- | --- | --- |
| `account` | 組織帳戶直接提供的授權來源；使用 `accountLicenseType` 時 request body 必須使用此來源。[Microsoft Learn：AccessLevel](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#accesslevel) | Stakeholder、Basic、Basic + Test Plans 的正常來源；**常用** |
| `msdn` | Visual Studio／MSDN 訂閱來源；使用 `msdnLicenseType` 時 request body 必須使用此來源。[Microsoft Learn：AccessLevel](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#accesslevel) | 只用於具備有效 Visual Studio 訂閱的人員 |
| `gitHub` | GitHub 授權來源；7.2-preview schema 規定 `gitHubLicenseType` 要搭配此來源。[Microsoft REST 7.2-preview schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1500-L1534) | GitHub Enterprise 權益由 Azure DevOps 在使用者登入後自動偵測，不應當成一般付費席次手動指派。[Microsoft：Group rules FAQ](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/assign-access-levels-by-group-membership?view=azure-devops#faqs) |
| `none` | schema sentinel；官方沒有把它映射成可用的產品存取層級。[Microsoft REST 7.2-preview schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1524-L1567) | 不公開為一般 CLI 選項 |
| `profile` | schema 有此值，但官方 REST 文件未提供語意、合法組合或寫入範例。[Microsoft Learn：REST 7.1 LicensingSource](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#licensingsource) | 查無足夠資料；視為服務端／相容性值 |
| `auto` | schema 有此值，但官方 REST 文件未提供語意、合法組合或寫入範例。[Microsoft Learn：REST 7.1 LicensingSource](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#licensingsource) | 查無足夠資料；視為服務端／相容性值 |
| `trial` | schema 有此值，但官方 REST 文件未定義期限、權益、可用組合或寫入範例。[Microsoft Learn：REST 7.1 LicensingSource](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#licensingsource) | 不應只從名稱推定其行為；視為服務端／相容性值 |

**`profile`、`auto`、`trial` 的名稱不能代替官方行為定義；目前查無足夠官方資料證明可安全由 `set-access` 寫入。**

* * *

## Visual Studio／MSDN 與 GitHub 授權型別

### `MsdnLicenseType`

REST schema 列出：

- `none`
- `eligible`
- `professional`
- `platforms`
- `testProfessional`
- `premium`
- `ultimate`
- `enterprise`

完整列舉與 `licensingSource=msdn` 的組合要求見 [Microsoft REST 7.2-preview schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1569-L1617)。

現行官方公開映射只明確建議下列組合：

| 使用者可見層級 | `accountLicenseType` | `licensingSource` | `msdnLicenseType` | 使用情境 |
| --- | --- | --- | --- | --- |
| Visual Studio Subscriber | `none` | `msdn` | `eligible` | 讓 Azure DevOps 驗證並套用使用者現有的 Visual Studio 訂閱；適用時可列為常用 |
| Visual Studio Enterprise subscription | `none` | `msdn` | `enterprise` | 有效 Visual Studio Enterprise 訂閱者 |

來源：[Microsoft：Programmatic mapping](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops#programmatic-mapping-of-access-levels)

訂閱權益方面，Visual Studio Professional 提供 Azure Boards 與 Repos 的 Basic 權益；Visual Studio Enterprise、Visual Studio Test Professional 與 MSDN Platforms 另含 Azure Test Plans。[Microsoft：Azure DevOps for Visual Studio subscribers](https://learn.microsoft.com/en-us/visualstudio/subscriptions/vs-azure-devops#eligibility)

`professional`、`platforms`、`testProfessional` 可由名稱對應到官方仍列出的訂閱產品，但現行程式化映射建議使用 `eligible` 讓 Azure DevOps 自動辨識訂閱。`premium` 與 `ultimate` 留存在 API enum，現行 Microsoft Learn 權益表沒有定義其目前可指派行為；**不應把這些 raw 值直接當成一般 `set-access` 選項。**

### `GitHubLicenseType`

REST 7.2-preview schema 只有：

- `none`
- `enterprise`

使用 `enterprise` 時，schema 要求 `licensingSource=gitHub`。[Microsoft REST 7.2-preview schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1500-L1519)

GitHub Enterprise 使用者登入 Azure DevOps 後會被自動辨識並取得等同 Basic 的存取層級；更新可能需要最多 24 小時，而且群組規則不能指派 GitHub Enterprise 存取層級。[Microsoft：Assign access levels with group rules](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/assign-access-levels-by-group-membership?view=azure-devops#faqs)

**因此 GitHub Enterprise 應顯示於查詢結果與說明文件，但不適合作為保證可由管理者手動授予的 `set-access` 選項。**

* * *

## `AssignmentSource` 全部列舉值

REST schema 列出 `none`、`unknown`、`groupRule`。[Microsoft Learn：REST 7.1 AssignmentSource](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#assignmentsource)

| REST 值 | 可驗證語意 | CLI 處理建議 |
| --- | --- | --- |
| `groupRule` | 授權由群組規則而來；群組規則會在多個規則之間提供最高的存取層級。[Microsoft：Assign access levels with group rules](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/assign-access-levels-by-group-membership?view=azure-devops#access-level-changes) | 查詢時顯示；不可由 `set-access` 偽造 |
| `unknown` | 官方 PATCH 範例在直接更新 access level 後，回應的 `assignmentSource` 為 `unknown`；官方未再提供更細的語意。[Microsoft：Update User Entitlement 範例](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/update-user-entitlement?view=azure-devops-rest-7.1#examples) | 視為服務端結果；不要解讀成錯誤 |
| `none` | schema sentinel；官方沒有定義額外語意。[Microsoft Learn：REST 7.1 AssignmentSource](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#assignmentsource) | 視為服務端結果 |

當使用者同時有直接指派與群組規則，群組規則若提供較高層級，Azure DevOps 會採用較高層級；若要完全由群組規則管理，官方要求移除直接指派。[Microsoft：Remove direct assignments](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/assign-access-levels-by-group-membership?view=azure-devops#remove-direct-assignments)

這表示 `set-access` 的成功回應不一定等於使用者最終有效層級被降低；CLI 若要保證說明正確，更新後應重新查詢並顯示實際 `assignmentSource` 與有效 access level。

* * *

## `AccountUserStatus` 全部列舉值

`status` 可回傳 `none`、`active`、`disabled`、`deleted`、`pending`、`expired`、`pendingDisabled`。官方 REST 文件對其語意定義如下：[Microsoft Learn：AccountUserStatus](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#accountuserstatus)

| 值 | 官方語意摘要 |
| --- | --- |
| `none` | 未定義狀態 |
| `active` | 使用者至少登入過一次組織 |
| `disabled` | 管理者停用，使用者不能登入 |
| `deleted` | 管理者已從組織移除使用者 |
| `pending` | 已邀請，但尚未註冊或登入 |
| `expired` | 授權已過期但仍在寬限期，可登入 |
| `pendingDisabled` | 待啟用的使用者被停用；重新啟用後仍回到 Pending |

這些值描述帳戶狀態，不是 access level 選項；`user set-access` 不應混用。

* * *

## 建議給 `adoctl` 的公開選項設計

### 一般模式

建議把「現行公開、直接指派且有完整官方語意」列為正常選項：

| CLI 值 | Request 組合 | 文件標籤 |
| --- | --- | --- |
| `basic` | `express` + `account` | **常用** |
| `stakeholder` | `stakeholder` + `account` | **常用** |
| `basic-test-plans` | `advanced` + `account` | **常用於測試與 QA** |
| `visual-studio-subscriber` | `none` + `msdn` + `eligible` | 有有效 Visual Studio 訂閱時使用 |
| `visual-studio-enterprise` | `none` + `msdn` + `enterprise` | 有有效 Visual Studio Enterprise 訂閱時使用 |

直接計費層級的成本注意事項：Stakeholder 免費；Basic 前五名使用者免費，第六名起計費；Basic + Test Plans 為付費層級，可免費試用 30 天。[Microsoft：Manage paid access for users](https://learn.microsoft.com/en-us/azure/devops/organizations/billing/buy-basic-access-add-users?view=azure-devops#assign-basic-or-basic--test-plans)

### Raw／相容性模式

如果產品需求堅持「接受 REST schema 的所有原始值」，應以明確的進階模式隔離，並在執行前標示：

- `earlyAdopter`：Microsoft 內部值，不保證一般組織可用。
- `professional`：保留值，現行官方映射查無完整語意。
- `none`：組合用 sentinel，不能單獨代表一個可用層級。
- `profile`、`auto`、`trial`：schema 值，但查無足夠官方資料證明合法寫入組合。
- `premium`、`ultimate`：歷史訂閱 enum；現行權益與可寫入行為未文件化。

**單純把 REST enum 全部平鋪成 `--access-level` 選項會混淆「產品存取層級」、「授權來源」與「服務端相容性值」，也可能造成無法預測的計費或 API 錯誤。**

* * *

## 版本與文件落差

- 專案目前 Member Entitlement Management client 使用 `7.1-preview.4`。
- Microsoft Learn 的公開 REST 7.1 contract 沒有 `gitHubLicenseType`，`LicensingSource` 也沒有 `gitHub`。[Microsoft Learn：REST 7.1 AccessLevel](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1#accesslevel) [Microsoft REST 7.1 OpenAPI schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.1/memberEntitlementManagement.json#L1416-L1580)
- 在 Microsoft 官方 REST 7.2 OpenAPI 檔案中，頂層版本是 `7.2-preview`，單一使用者 PATCH operation 指定的 API version 是 `7.2-preview.5`；該版本才把 `gitHubLicenseType` 與 `licensingSource=gitHub` 納入 `AccessLevel` contract。[Microsoft REST 7.2-preview.5 Update User Entitlement](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/update-user-entitlement?view=azure-devops-rest-7.2) [Microsoft REST 7.2-preview OpenAPI schema](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json#L1431-L1617)
- 查無 Microsoft 官方 contract 或範例可證實 `7.1-preview.4` 接受 `licensingSource=gitHub` 或 `gitHubLicenseType`。7.2-preview.5 的 PATCH 頁面雖已列出欄位與 request body 組合要求，官方範例仍只示範 `express/account`，沒有 GitHub Enterprise 的 PATCH request 範例。
- Azure CLI `az devops user update` 接受 `advanced`、`earlyAdopter`、`express`、`professional`、`stakeholder`，且其官方原始碼只把 `accountLicenseType` 寫入 `/accessLevel`，沒有提供完整的 Visual Studio／GitHub 複合欄位介面。[Microsoft：az devops user](https://learn.microsoft.com/en-us/cli/azure/devops/user?view=azure-cli-latest#az-devops-user-update) [Azure CLI extension 原始碼](https://github.com/Azure/azure-devops-cli-extension/blob/ad7deee2ae0a9feb9c8d2a14774ff5690b287bc5/azure-devops/azext_devops/dev/team/user.py#L54-L74)

**目前 `adoctl` 不可宣稱能以 `7.1-preview.4` 設定 GitHub Enterprise。若未先升級至 `7.2-preview.5` 並以具備真實 GitHub Enterprise 權益的測試帳號驗證，GitHub 權益只能列為可查詢／自動偵測的狀態。**

* * *

## 查核來源

- [Microsoft Learn：About access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops)
- [Microsoft Learn：User Entitlements REST API 7.1](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements?view=azure-devops-rest-7.1)
- [Microsoft 官方 REST API 7.2-preview 規格](https://github.com/MicrosoftDocs/vsts-rest-api-specs/blob/3785641890ef409f82134f5d2fcccb2b2631ab9c/specification/memberEntitlementManagement/7.2/memberEntitlementManagement.json)
- [Microsoft Learn：az devops user](https://learn.microsoft.com/en-us/cli/azure/devops/user?view=azure-cli-latest)
- [Azure 官方 Azure DevOps CLI extension](https://github.com/Azure/azure-devops-cli-extension/blob/ad7deee2ae0a9feb9c8d2a14774ff5690b287bc5/azure-devops/azext_devops/dev/team/user.py)
- [Microsoft 官方 Terraform provider：user_entitlement](https://github.com/microsoft/terraform-provider-azuredevops/blob/b00216db479c11cae8e7cfec42a79c29af853898/website/docs/r/user_entitlement.html.markdown)
- [Microsoft Learn：Assign access levels with group rules](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/assign-access-levels-by-group-membership?view=azure-devops)
- [Microsoft Learn：Manage paid access for users](https://learn.microsoft.com/en-us/azure/devops/organizations/billing/buy-basic-access-add-users?view=azure-devops)
- [Microsoft Learn：Azure DevOps for Visual Studio subscribers](https://learn.microsoft.com/en-us/visualstudio/subscriptions/vs-azure-devops)
