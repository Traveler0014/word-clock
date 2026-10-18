/* 时区表：配网页（自动识别）与管理页（下拉选择）共用。
 *
 * 为什么以 POSIX 串为主键：ESP-IDF 的 newlib 只认 POSIX（无 zoneinfo 数据库），
 * 所以设备侧只存 POSIX 串；IANA 名仅用于"从手机浏览器识别"和"下拉展示"。
 *
 * 每一行 = 一个 POSIX 串 + 它覆盖的 IANA 名 + 双语友好名。
 * 同一 POSIX 串的别名（如 Asia/Shanghai 与 Asia/Taipei）合并在一行，避免下拉里重复。
 */
window.TZ_TABLE = [
  { posix: "CST-8", offset: "UTC+8",
    zh: "北京 · 新加坡 · 台北 · 珀斯", en: "Beijing · Singapore · Taipei · Perth",
    iana: ["Asia/Shanghai", "Asia/Chongqing", "Asia/Harbin", "Asia/Urumqi", "Asia/Hong_Kong",
           "Asia/Macau", "Asia/Taipei", "Asia/Singapore", "Asia/Kuala_Lumpur", "Asia/Manila",
           "Australia/Perth", "Asia/Brunei"] },

  { posix: "JST-9", offset: "UTC+9",
    zh: "东京 · 首尔 · 平壤", en: "Tokyo · Seoul · Pyongyang",
    iana: ["Asia/Tokyo", "Asia/Seoul", "Asia/Pyongyang", "Asia/Yakutsk"] },

  { posix: "ACST-9:30", offset: "UTC+9:30",
    zh: "达尔文", en: "Darwin",
    iana: ["Australia/Darwin"] },

  { posix: "ACST-9:30ACDT,M10.1.0,M4.1.0/3", offset: "UTC+9:30",
    zh: "阿德莱德（含夏令时）", en: "Adelaide (DST)",
    iana: ["Australia/Adelaide"] },

  { posix: "AEST-10", offset: "UTC+10",
    zh: "布里斯班", en: "Brisbane",
    iana: ["Australia/Brisbane", "Australia/Lindeman"] },

  { posix: "AEST-10AEDT,M10.1.0,M4.1.0/3", offset: "UTC+10",
    zh: "悉尼 · 墨尔本（含夏令时）", en: "Sydney · Melbourne (DST)",
    iana: ["Australia/Sydney", "Australia/Melbourne", "Australia/Hobart",
           "Australia/Canberra", "Australia/ACT"] },

  { posix: "NZST-12NZDT,M9.5.0,M4.1.0/3", offset: "UTC+12",
    zh: "奥克兰 · 惠灵顿（含夏令时）", en: "Auckland · Wellington (DST)",
    iana: ["Pacific/Auckland", "Pacific/Wellington"] },

  { posix: "FJT-12", offset: "UTC+12",
    zh: "斐济", en: "Fiji",
    iana: ["Pacific/Fiji"] },

  { posix: "ICT-7", offset: "UTC+7",
    zh: "曼谷 · 雅加达 · 河内", en: "Bangkok · Jakarta · Hanoi",
    iana: ["Asia/Bangkok", "Asia/Jakarta", "Asia/Ho_Chi_Minh", "Asia/Saigon",
           "Asia/Phnom_Penh", "Asia/Vientiane", "Asia/Krasnoyarsk"] },

  { posix: "BDT-6", offset: "UTC+6",
    zh: "达卡 · 阿拉木图", en: "Dhaka · Almaty",
    iana: ["Asia/Dhaka", "Asia/Almaty", "Asia/Dushanbe"] },

  { posix: "IST-5:30", offset: "UTC+5:30",
    zh: "新德里 · 孟买 · 科伦坡", en: "New Delhi · Mumbai · Colombo",
    iana: ["Asia/Kolkata", "Asia/Calcutta", "Asia/Colombo"] },

  { posix: "NPT-5:45", offset: "UTC+5:45",
    zh: "加德满都", en: "Kathmandu",
    iana: ["Asia/Kathmandu"] },

  { posix: "PKT-5", offset: "UTC+5",
    zh: "卡拉奇 · 塔什干", en: "Karachi · Tashkent",
    iana: ["Asia/Karachi", "Asia/Tashkent", "Asia/Yekaterinburg"] },

  { posix: "GST-4", offset: "UTC+4",
    zh: "迪拜 · 阿布扎比 · 第比利斯", en: "Dubai · Abu Dhabi · Tbilisi",
    iana: ["Asia/Dubai", "Asia/Muscat", "Asia/Baku", "Asia/Tbilisi", "Asia/Yerevan"] },

  { posix: "IRST-3:30", offset: "UTC+3:30",
    zh: "德黑兰", en: "Tehran",
    iana: ["Asia/Tehran"] },

  { posix: "MSK-3", offset: "UTC+3",
    zh: "莫斯科 · 伊斯坦布尔 · 内罗毕", en: "Moscow · Istanbul · Nairobi",
    iana: ["Europe/Moscow", "Europe/Istanbul", "Asia/Riyadh", "Asia/Qatar",
           "Asia/Kuwait", "Africa/Nairobi", "Africa/Addis_Ababa", "Africa/Kampala"] },

  { posix: "EET-2EEST,M3.5.0/3,M10.5.0/4", offset: "UTC+2",
    zh: "雅典 · 赫尔辛基 · 开罗 · 基辅（含夏令时）", en: "Athens · Helsinki · Cairo · Kyiv (DST)",
    iana: ["Europe/Athens", "Europe/Helsinki", "Europe/Kiev", "Europe/Kyiv",
           "Europe/Bucharest", "Europe/Sofia", "Africa/Cairo", "Asia/Beirut",
           "Asia/Jerusalem", "Europe/Riga", "Europe/Tallinn", "Europe/Vilnius"] },

  { posix: "CAT-2", offset: "UTC+2",
    zh: "约翰内斯堡 · 哈拉雷", en: "Johannesburg · Harare",
    iana: ["Africa/Johannesburg", "Africa/Harare", "Africa/Maputo"] },

  { posix: "CET-1CEST,M3.5.0,M10.5.0/3", offset: "UTC+1",
    zh: "巴黎 · 柏林 · 罗马 · 马德里（含夏令时）", en: "Paris · Berlin · Rome · Madrid (DST)",
    iana: ["Europe/Paris", "Europe/Berlin", "Europe/Rome", "Europe/Madrid",
           "Europe/Amsterdam", "Europe/Zurich", "Europe/Stockholm", "Europe/Vienna",
           "Europe/Warsaw", "Europe/Prague", "Europe/Budapest", "Europe/Copenhagen",
           "Europe/Oslo", "Europe/Brussels", "Europe/Luxembourg", "Europe/Belgrade"] },

  { posix: "WAT-1", offset: "UTC+1",
    zh: "拉各斯 · 金沙萨", en: "Lagos · Kinshasa",
    iana: ["Africa/Lagos", "Africa/Kinshasa", "Africa/Algiers", "Africa/Tunis"] },

  { posix: "GMT0BST,M3.5.0/1,M10.5.0", offset: "UTC+0",
    zh: "伦敦（含夏令时）", en: "London (DST)",
    iana: ["Europe/London", "Europe/Belfast"] },

  { posix: "GMT0IST,M3.5.0/1,M10.5.0", offset: "UTC+0",
    zh: "都柏林（含夏令时）", en: "Dublin (DST)",
    iana: ["Europe/Dublin"] },

  { posix: "WET0WEST,M3.5.0/1,M10.5.0", offset: "UTC+0",
    zh: "里斯本（含夏令时）", en: "Lisbon (DST)",
    iana: ["Europe/Lisbon"] },

  { posix: "UTC0", offset: "UTC+0",
    zh: "协调世界时 · 雷克雅未克", en: "Coordinated Universal Time · Reykjavik",
    iana: ["UTC", "Etc/UTC", "Etc/GMT", "GMT", "Atlantic/Reykjavik"] },

  { posix: "-03", offset: "UTC−3",
    zh: "圣保罗 · 布宜诺斯艾利斯", en: "Sao Paulo · Buenos Aires",
    iana: ["America/Sao_Paulo", "America/Argentina/Buenos_Aires", "America/Montevideo",
           "America/Recife", "America/Fortaleza"] },

  { posix: "AST4ADT,M3.2.0,M11.1.0", offset: "UTC−4",
    zh: "哈利法克斯 · 圣地亚哥（含夏令时）", en: "Halifax · Santiago (DST)",
    iana: ["America/Halifax", "America/Santiago", "America/Caracas"] },

  { posix: "EST5EDT,M3.2.0,M11.1.0", offset: "UTC−5",
    zh: "纽约 · 多伦多 · 华盛顿（含夏令时）", en: "New York · Toronto · Washington (DST)",
    iana: ["America/New_York", "America/Toronto", "America/Detroit", "America/Miami",
           "America/Bogota", "America/Lima"] },

  { posix: "CST6CDT,M3.2.0,M11.1.0", offset: "UTC−6",
    zh: "芝加哥 · 达拉斯（含夏令时）", en: "Chicago · Dallas (DST)",
    iana: ["America/Chicago", "America/Winnipeg", "America/Dallas"] },

  { posix: "CST6", offset: "UTC−6",
    zh: "墨西哥城", en: "Mexico City",
    iana: ["America/Mexico_City", "America/Guatemala", "America/Costa_Rica"] },

  { posix: "MST7MDT,M3.2.0,M11.1.0", offset: "UTC−7",
    zh: "丹佛 · 卡尔加里（含夏令时）", en: "Denver · Calgary (DST)",
    iana: ["America/Denver", "America/Edmonton"] },

  { posix: "MST7", offset: "UTC−7",
    zh: "菲尼克斯", en: "Phoenix",
    iana: ["America/Phoenix"] },

  { posix: "PST8PDT,M3.2.0,M11.1.0", offset: "UTC−8",
    zh: "洛杉矶 · 温哥华（含夏令时）", en: "Los Angeles · Vancouver (DST)",
    iana: ["America/Los_Angeles", "America/Vancouver", "America/Tijuana"] },

  { posix: "AKST9AKDT,M3.2.0,M11.1.0", offset: "UTC−9",
    zh: "安克雷奇（含夏令时）", en: "Anchorage (DST)",
    iana: ["America/Anchorage"] },

  { posix: "HST10", offset: "UTC−10",
    zh: "檀香山", en: "Honolulu",
    iana: ["Pacific/Honolulu"] }
];

/* 按 IANA 名查表（手机浏览器给的就是 IANA） */
window.tzLookupIana = function (iana) {
  if (!iana) return null;
  for (var i = 0; i < window.TZ_TABLE.length; i++) {
    if (window.TZ_TABLE[i].iana.indexOf(iana) >= 0) return window.TZ_TABLE[i];
  }
  return null;
};

/* 按 POSIX 串查表（设备里存的是 POSIX） */
window.tzLookupPosix = function (posix) {
  for (var i = 0; i < window.TZ_TABLE.length; i++) {
    if (window.TZ_TABLE[i].posix === posix) return window.TZ_TABLE[i];
  }
  return null;
};

/* 由分钟偏移生成 POSIX 串：POSIX 的符号与 UTC 偏移相反 */
window.tzPosixFromOffset = function (minEast) {
  if (minEast === 0) return "UTC0";
  var sign = minEast > 0 ? "-" : "";
  var a = Math.abs(minEast);
  var h = Math.floor(a / 60), m = a % 60;
  return "TZ" + sign + h + (m ? ":" + (m < 10 ? "0" + m : m) : "");
};

window.tzOffsetLabel = function (minEast) {
  var sign = minEast >= 0 ? "+" : "−";
  var a = Math.abs(minEast);
  var h = Math.floor(a / 60), m = a % 60;
  return "UTC" + sign + h + (m ? ":" + (m < 10 ? "0" + m : m) : "");
};

/* 自动识别：IANA 命中就用表里的 POSIX（含夏令时规则）；否则退化为当前固定偏移 */
window.tzDetect = function () {
  try {
    var iana = (Intl.DateTimeFormat().resolvedOptions().timeZone) || "";
    var hit = window.tzLookupIana(iana);
    if (hit) return { posix: hit.posix, offset: hit.offset, label: iana, known: true };
    var off = -new Date().getTimezoneOffset();
    return {
      posix: window.tzPosixFromOffset(off),
      offset: window.tzOffsetLabel(off),
      label: iana || window.tzOffsetLabel(off),
      known: false
    };
  } catch (e) {
    return { posix: "CST-8", offset: "UTC+8", label: "CST-8", known: false };
  }
};
