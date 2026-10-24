# GRKit

僅適用於 RICOH GR III 與 GR III HDF 韌體 2.10。

## 安裝

1. 關閉相機。
2. 把 `SD_ROOT` 中的內容複製到記憶卡根目錄。
3. 插入記憶卡。按住 MENU 按鈕開機。等待相機自動關機。
4. 正常開機。

第 3 步會開啟相機的 SD 腳本功能。相機先顯示 Mode Set Command，接著顯示 Complete，之後自動關機。

![Mode Set Command 畫面顯示 Script - Enable 和 Complete。](images/modeset.png)

如果 SD 腳本功能已經開啟，跳過第 3 步。
安裝完成後，你可以從記憶卡根目錄刪除 `00078350.588` 與 `DEVELOP.MOD`。這一步是選用的。

## 更新

1. 關閉相機。
2. 把新安裝包中 `SD_ROOT` 的內容複製到記憶卡根目錄。取代或合併現有檔案都可以。
3. 正常開機。

GRKit 把設定和效果保存在 `GRKIT` 資料夾中，所以更新後它們仍在。不需要先刪除 `SCRIPT`。如果從 GRKit 1.2 或更早版本更新，並取代了 `SCRIPT` 資料夾（例如使用 macOS Finder 預設的「取代」），之前的設定，以及新安裝包中沒有的效果，會遺失，因為 GRKit 1.2 把它們保存在 `SCRIPT` 中。

## 使用

在拍攝模式下按下 ADJ 桿進入 ADJ 模式，然後選擇第 6 個功能 GRK。GRKit 在開機幾秒後才載入，所以 GRK 可能稍後才出現。載入完成後，拍攝畫面頂部會顯示 **GRKit** 幾秒。這個提示需要畫幅或輔助線模組。

![GRKit 頁面列出模組，並顯示目前的效果。](images/grkit-root.jpg)

照片：nagi usano 拍攝的 [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)，[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

- **影像控制（Image Control）：** 反白一個效果，即可在實時顯示中預覽。按 OK 按鈕保留這個效果。按 Fn 按鈕調整這個效果的 [標準] 參數。在 GRK 選單的影像控制一列按 Fn 按鈕，可以調整目前選擇的效果。每個效果單獨保存自己的調整，不會改變相機的影像控制設定。
- **畫幅（Framing）：** 選擇 XPan、2.35:1、16:9、4:3、5:4 或 1:1。1:1 使用相機內建的 1:1 [長寬比]。選擇其他畫幅時，相機的 [長寬比] 會設為 3:2，顯示屏上照片以外的區域會變暗。DNG 檔案保留完整的感光元件影像，並記錄裁切範圍。套用這個裁切範圍的 RAW 軟體只顯示照片區域。
- **輔助線（Guides）：** 選擇一種輔助線樣式。輔助線只顯示在顯示屏上，不會出現在照片中。
- **加印日期攝像（Date Imprint）：** 按 OK 按鈕開啟設定。在 **日期樣式（Date Style）** 中選擇一種日期樣式。在 **日期字體（Date Look）** 中選擇數字的字體：經典、GR1s 或 GR II。照片右下角會加印日期。
- **粒紋感（Grain）：** 選擇弱、中或強。粒紋感只加在照片上，不影響影片。

反白最後一行 GRK，即可看到 GRKit 的版本號。

按 MENU 按鈕返回上一層。關機後 GRKit 會保留你的設定。

GRKit 把效果保存在記憶卡的 `GRKIT/EFFECTS` 資料夾中。新的安裝包會加入它的效果，並取代這些效果的舊版本，但不會移除其他效果。要移除一個效果，先關閉相機，然後從 `GRKIT/EFFECTS` 刪除它的檔案。只有當之後的安裝包中這個效果的檔案有變化時，它才會被重新加入。`GRKIT/EFFECTS.TXT` 列出每個效果的檔案。GRKit 最多安裝 20 個效果：先安裝最近一個安裝包的效果，再安裝其他效果。

## 疑難排解

### 按住 MENU 按鈕開機時，相機開始更新韌體

記憶卡根目錄中有韌體更新檔案。關閉相機。從記憶卡根目錄刪除所有 `.bin` 檔案。然後重新執行第 3 步。

### 相機沒有顯示 Mode Set Command

- 先按住 MENU 按鈕，再開機。如果開機後才按 MENU 按鈕，SD 腳本功能不會開啟。
- 把 `SD_ROOT` 中的內容複製到記憶卡根目錄，不要複製到 `DCIM` 或其他資料夾中。

### ADJ 模式中沒有 GRK

1. 開機後等待幾秒。GRKit 在相機啟動後才載入。載入完成後，拍攝畫面頂部會顯示 **GRKit** 幾秒。
2. 在 ADJ 模式中找 GRK。GRK 不在 MENU 選單中。
3. 如果第 3 步中相機沒有顯示 Mode Set Command，重新執行第 3 步。
4. 確認記憶卡根目錄中有 `SCRIPT` 資料夾。複製 `SD_ROOT` 中的全部內容，包括 `SCRIPT`。
5. 如果記憶卡中有 `SCRIPT/PLUGINS/INSTALL.ERR`，表示 GRKit 沒有安裝成功。確認套件與相機的韌體版本對應。在相機選單中可以查看韌體版本。如果版本正確，請提交[裝置報告](https://github.com/ahonn/grkit/issues/new?template=device-report.yml)，並附上 `INSTALL.ERR` 中的全部內容。
6. GRKit 只能在 Ricoh 官方韌體上執行，不能在修改過的韌體上執行。

## 移除或復原

要移除 GRKit，先關閉相機。然後從記憶卡刪除 `SCRIPT` 和 `GRKIT` 資料夾。`GRKIT` 保存 GRKit 的設定。

SD 腳本設定會保持開啟。要關閉它：

1. 在 https://grkit.app 下載你的相機對應的關閉 SD 腳本的檔案。按鈕在安裝步驟下方。
2. 把 `SD_ROOT` 中的內容複製到記憶卡根目錄。
3. 插入記憶卡。按住 MENU 按鈕開機，等待相機自動關機。
4. 從記憶卡根目錄刪除 `00078350.588` 和 `DEVELOP.MOD`。

如果相機在啟動時停止回應，取出電池與記憶卡。然後不插記憶卡開機。如果仍然無法啟動，請參閱[復原指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hant.md)。

GRKit 與 Ricoh 無關。原始碼與授權：https://github.com/ahonn/grkit
