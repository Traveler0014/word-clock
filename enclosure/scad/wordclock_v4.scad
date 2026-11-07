// =============================================================================
// Word Clock 结构件 v4 —— 适配 192×202 灯板（**V-CUT 工艺边不掰断**）
// -----------------------------------------------------------------------------
// 与 v3 (wordclock_pcb.scad) 的差别：
//   1) 尺寸全部按 hardware/fab/wordclock-led.drl 实测重算：
//      灯板 192(X)×202(Y)×2.0；灯阵 11×10 @pitch15 居中（gx 13.5 / gy 26）
//      角点灯 (7,12)/(185,12)/(7,190)/(185,190)
//      安装孔 (6,101)/(186,101)/(96,11)/(96,191)
//      控制区：J2@X24、J1@X124（针 Y 181.4…189.0）、C117@(148.5,185.5) Ø10.5 高 12
//   2) **前面板 + 格栅合并一体**（front_black），扩散层改为**同件打印膜**
//      （front_white，AMS 双料，全程仅 2 次换料）→ 不再裁膜、不再贴膜
//   3) 外形 210×210（方正），腔体 204×204，面板 214.8×214.8，总厚 34.8
//   4) 接口：**背面 = USB-C 供电母插**（低、居中）；
//           **右侧墙 = DevKit 双 USB-C**（一个开口 + 盖板）；底边完全干净（竖放接触面）
//   5) 灯板定位：4 个 Ø7 台阶柱（Ø3 销入 Ø3.2 孔），由前面板壁端压紧
//
// 用法 part = "body" / "front_black" / "front_white" / "usb_cover"
//                / "coupon" / "snap_test" / "all" / "check"
//   打印：
//     body        背板朝下，无支撑（腔体/开孔/卡座全部朝上）
//     front_black 正面朝下（PEI 纹理），无支撑
//     front_white 不要单独打！在切片软件里与 front_black 合并为一个 object 的
//                 两个 part，分别指定黑 / 白(半透) 料槽
//     coupon      3×3 格试样（≈48×48），先验证亮度 / 热斑 / 串光
//
// 坐标约定（与灯板一一对应）：
//   原点 = 灯板(PCB) 左下角；+X 向右，+Y 向上（正视），+Z 朝观察者（出光方向）
//   灯板:   Z = -2.0 … 0       (0 = 灯板正面)
//   格壁:   Z =  0   … 14.0    壁端落在灯板正面
//   打印膜: Z = 14.0 … 14.8    (白色，整幅)
//   面板:   Z = 14.8 … 16.4    (字母凹窗深 1.6)
//   电子仓: Z = -16.0 … -2.0   (bay 14；母插处局部凹坑到 -17.0)
//   背板:   Z = -18.4 … -16.0
// =============================================================================

part = "all";
// ["body","front_black","front_white","usb_cover","coupon_black","coupon_white",
//  "snap_test","all","check"]

$fn = 48;

// ============================ 灯板（drill 实测） ============================
bx = 192.0;  by = 202.0;  bt = 2.0;
bcx = bx / 2;  bcy = by / 2;              // 96, 101

pitch = 15.0;  cols = 11;  rows = 10;
gx = (bx - cols * pitch) / 2;             // 13.5
gy = (by - rows * pitch) / 2;             // 26.0

dot_dx = 7.0;  dot_dy = 12.0;             // 角点灯距边：X 7 / Y 12（工艺边所致）
dot_blk = 13.0;  dot_well = 9.0;  dot_win = 6.0;

mt_dx = 6.0;  mt_dy = 11.0;               // 安装孔距边：X 6 / Y 11

j2_x = 24.0;  j1_x = 124.0;               // 1×4 排针 X
hdr_y0 = 181.4;  hdr_dy = 2.54;           // 4 针 Y（自下而上）
hdr_h = 8.0;                              // 排针座高（相对板面）
cap_x = 148.5;  cap_y = 185.5;
cap_d = 10.5;  cap_h = 12.0;              // 电解高（相对板面，实测）
tail_h = 3.0;                             // 背面尾针长（P1 焊线点）

// USB-C 供电母插（14×20 转接板；出线轴沿 14mm 短边、与板面共面 → 只能竖立安装）
pwr_x = 96.0;  pwr_y = 30.0;
pwr_dep = 14.0;                           // 沿出线方向的板长
pwr_wid = 20.0;                           // 跨出线方向的板宽
pwr_hole_w = 9.6;  pwr_hole_h = 5.2;      // 背板开孔（X × Y）

// ESP32-C5-DevKitC-1：25.4×61.7×2.0，器件最高 6.4 → 连板 8.4
dk_w = 25.4;  dk_l = 61.7;  dk_h = 8.4;
dk_x0 = 135.6;                            // 长轴沿 X、短边(USB) 贴右墙（板缘 197.3，墙内面 198）
dk_y0 = 55.0;
dk_usb1 = 5.5;  dk_usb2 = 19.9;           // 两个 USB-C 中心（自短边一端）
dk_usb_w = 10.0;  dk_usb_h = 5.0;
dk_usb_z = -12.4;                         // 开口中心 Z（板贴背板、器件朝 +Z）

// ================================ Z 栈 ================================
h_cell    = 14.0;                         // 格壁长度 = 出光距离
m_t       = 1.2;                          // 打印扩散膜厚度
face_t    = 1.2;                          // 面板前层（黑）
cut_into_m = 0.4;                         // 字母/角点窗再切进膜 0.4（≥2 层）→ 见下方注释
Z_pcb_b   = -bt;                          // -2.0
Z_sheet_b = h_cell;                       // 14.0  格壁顶 / 膜底
Z_sheet_t = Z_sheet_b + m_t;              // 14.8  膜顶 / 面板底 = 壳体前缘
Z_face_t  = Z_sheet_t + face_t;           // 16.4  型面

bay   = 14.0;
Z_bay = Z_pcb_b - bay;                    // -16.0 背板内面
back_t = 2.4;
Z_bk  = Z_bay - back_t;                   // -18.4 背板外面

// ============================ 外形 / 格网 ============================
shell_outer = 210.0;
cavity      = 204.0;
shell_r     = 5.0;
cav_wall    = (shell_outer - cavity) / 2; // 3.0
skirt_t     = 2.4;
face_outer  = shell_outer + 2 * skirt_t;  // 214.8
grid_wall   = 1.2;
grid_cell   = pitch - grid_wall;          // 13.8 名义（实测打印 13.6）
sheet_side  = 200.0;                      // 膜外廓（小于腔体 204 的两倍多，避免从缝隙看到白边）

// ============================ 卡扣 ============================
skirt_cl  = 0.25;
skirt_bot = 5.0;                          // 侧裙下沿（Z 5.0…14.8）
groove_z  = 6.8;                          // 卡扣槽下沿
groove_h  = 2.8;
groove_d  = 0.6;
snap_d    = 0.75;                         // 一体件刚 → 过盈取小(0.15)
snap_h    = 2.6;
snap_len  = 6.0;
snap_z    = groove_z + 0.1;

// 字母（必须与 firmware/wordclock-core/src/lib.rs 一致）
LETTERS = [
  ["I","T","L","I","S","A","S","A","M","P","M"],
  ["A","C","Q","U","A","R","T","E","R","D","C"],
  ["T","W","E","N","T","Y","F","I","V","E","X"],
  ["H","A","L","F","S","T","E","N","F","T","O"],
  ["P","A","S","T","E","R","U","N","I","N","E"],
  ["O","N","E","S","I","X","T","H","R","E","E"],
  ["F","O","U","R","F","I","V","E","T","W","O"],
  ["E","I","G","H","T","E","L","E","V","E","N"],
  ["S","E","V","E","N","T","W","E","L","V","E"],
  ["T","E","N","S","E","O","C","L","O","C","K"],
];
font_name   = "Allerta Stencil";
ref_size    = 10.5;    // 下面 INK 表的测量基准尺寸
cap_target  = 10.0;    // 目标大写高（字面高）
max_w       = 11.6;    // 单字母最大宽：格 13.8 → 单边留 1.1

// Allerta Stencil 的字形墨迹包围盒（size=10.5 / halign=left / valign=baseline 实测）
// 注意：该字体的 size 参数 ≈ 大写高（不是 em），所以 cap_h_ratio 不能当 0.72 用；
//       W 的 stencil 字形宽达 13.5mm，必须单独做横向压缩，否则会啃到格壁。
GLYPHS = ["A","C","D","E","F","G","H","I","K","L","M","N","O",
          "P","Q","R","S","T","U","V","W","X","Y"];
INK = [
  [0.90, 9.91,  0.00, 10.48],   // A
  [1.12, 8.42, -0.07, 10.65],   // C
  [1.37, 9.84,  0.00, 10.50],   // D
  [1.35, 7.92,  0.00, 10.50],   // E
  [1.35, 7.92,  0.00, 10.50],   // F
  [1.12, 8.96, -0.07, 10.67],   // G
  [1.34, 9.33,  0.00, 10.50],   // H
  [1.34, 3.08,  0.00, 10.50],   // I
  [1.32, 8.43,  0.00, 10.50],   // K
  [1.32, 7.89,  0.00, 10.50],   // L
  [1.31,11.01,  0.00, 10.50],   // M
  [1.31, 8.90,  0.00, 10.50],   // N
  [1.10, 9.84, -0.07, 10.65],   // O
  [1.30, 7.98,  0.00, 10.50],   // P
  [0.90, 9.65, -2.41, 10.65],   // Q（有下伸尾，按墨迹居中上移）
  [1.28, 8.40,  0.00, 10.50],   // R
  [1.15, 7.25, -0.06, 10.67],   // S
  [0.73, 8.72,  0.00, 10.50],   // T
  [1.17, 9.33, -0.07, 10.50],   // U
  [0.85, 9.75, -0.06, 10.50],   // V
  [1.04,14.55, -0.06, 10.50],   // W ← 需横向压缩
  [1.05, 8.96,  0.00, 10.50],   // X
  [0.77, 8.59,  0.00, 10.50],   // Y
];
function ink_of(ch) = INK[search([ch], GLYPHS)[0]];

echo(str("板 ", bx, "×", by, "×", bt, "  外形 ", shell_outer, "×", shell_outer,
         "  总厚 ", Z_face_t - Z_bk, "mm  仓深 ", bay, "mm"));

// ============================== 2D 辅助 ==============================
function cell_x(c) = gx + (c + 0.5) * pitch;
function cell_y(r) = by - (gy + (r + 0.5) * pitch);   // r=0 在顶部

module rounded_sq(w, h, r) { offset(r = r) square([w - 2 * r, h - 2 * r], center = true); }

// 单个字形：按墨迹包围盒居中到 (x,y)；过宽的字（W）只做横向压缩，不改高度
module glyph_at(ch, x, y) {
    b = ink_of(ch);
    k  = cap_target / ref_size;
    w  = (b[1] - b[0]) * k;
    sx = (w > max_w) ? max_w / w : 1;
    translate([x, y]) scale([k * sx, k])
        translate([-(b[0] + b[1]) / 2, -(b[2] + b[3]) / 2])
            text(ch, size = ref_size, font = font_name,
                 halign = "left", valign = "baseline");
}

module letters_2d() {
    for (r = [0:rows - 1]) for (c = [0:cols - 1])
        glyph_at(LETTERS[r][c], cell_x(c), cell_y(r));
}

module cell_holes_2d() {
    for (r = [0:rows - 1]) for (c = [0:cols - 1])
        translate([cell_x(c), cell_y(r)])
            square([grid_cell, grid_cell], center = true);
}

module dot_positions() {
    for (x = [dot_dx, bx - dot_dx]) for (y = [dot_dy, by - dot_dy])
        translate([x, y]) children();
}

// 格网 2D：灯阵外框壁 + 110 格孔 + 4 角点方块（含 Ø9 光腔）
module lattice_2d() {
    difference() {
        translate([gx - 0.6, gy - 0.6])
            square([cols * pitch + 1.2, rows * pitch + 1.2]);
        cell_holes_2d();
    }
    dot_positions() difference() {
        translate([-dot_blk / 2, -dot_blk / 2]) square([dot_blk, dot_blk]);
        circle(d = dot_well);
    }
}

// ===================== 前面板 + 格栅（一体，黑色） =====================
module snap_hook(len, prot, h) {
    hull() {
        translate([0, -len / 2, 0]) cube([0.01, len, h]);
        translate([prot, -len / 2, prot]) cube([0.01, len, h - 2 * prot]);
    }
}

module hook_at(side, t) {
    if (side == 0) translate([bcx + shell_outer / 2 + skirt_cl, bcy + t, snap_z])
        rotate([0, 0, 180]) snap_hook(snap_len, snap_d, snap_h);
    if (side == 1) translate([bcx + t, bcy + shell_outer / 2 + skirt_cl, snap_z])
        rotate([0, 0, 270]) snap_hook(snap_len, snap_d, snap_h);
    if (side == 2) translate([bcx - shell_outer / 2 - skirt_cl, bcy + t, snap_z])
        snap_hook(snap_len, snap_d, snap_h);
    if (side == 3) translate([bcx + t, bcy - shell_outer / 2 - skirt_cl, snap_z])
        rotate([0, 0, 90]) snap_hook(snap_len, snap_d, snap_h);
}

module front_black() {
    difference() {
        union() {
            // 面板（实体边框遮住控制带）
            translate([bcx, bcy, Z_sheet_t])
                linear_extrude(face_t) rounded_sq(face_outer, face_outer, 6);
            // 侧裙（内面留 skirt_cl 包住壳体）
            translate([bcx, bcy, skirt_bot])
                linear_extrude(Z_sheet_t - skirt_bot + 0.01) difference() {
                    rounded_sq(face_outer, face_outer, 6);
                    rounded_sq(shell_outer + 2 * skirt_cl,
                               shell_outer + 2 * skirt_cl, 4);
                }
            // 卡扣 8 个
            for (s = [0:3]) for (t = [-56, 56]) hook_at(s, t);
            // 格网
            translate([0, 0, 0]) linear_extrude(Z_sheet_b + 0.01) lattice_2d();
        }
        // 字母凹窗：穿透黑面板 face_t + 再切进白膜 cut_into_m
        //   → 可见的扩散面 = 换料后的第 3 层，换料"染色"（第 1–2 层）被切掉/藏在不可见处
        translate([0, 0, Z_sheet_t - cut_into_m - 0.01])
            linear_extrude(face_t + cut_into_m + 0.02) letters_2d();
        // 角点圆窗（同样切进膜）
        dot_positions() translate([0, 0, Z_sheet_t - cut_into_m - 0.01])
            cylinder(d = dot_win, h = face_t + cut_into_m + 0.02);
    }
}

// 打印膜（白色，整幅；独立 body 以便切片软件指定料槽）
//   必须在**自己的几何里**切掉字母窗内的前 cut_into_m —— 切片时两个 part 各自独立，
//   黑色件上的"洞"不会减掉白色件的材料。被切掉的那两层正是换料后挤出的前两层。
module front_white() {
    difference() {
        translate([bcx, bcy, Z_sheet_b])
            linear_extrude(m_t) rounded_sq(sheet_side, sheet_side, 2);
        translate([0, 0, Z_sheet_t - cut_into_m - 0.01])
            linear_extrude(cut_into_m + 0.02) {
                letters_2d();
                dot_positions() circle(d = dot_win);
            }
    }
}

module front() { front_black(); front_white(); }

// ============================== 底壳 ==============================
module pcb_posts() {
    for (p = [[mt_dx, bcy], [bx - mt_dx, bcy],
              [bcx, mt_dy], [bcx, by - mt_dy]])
        translate([p[0], p[1], Z_bay]) {
            cylinder(d = 7.0, h = Z_pcb_b - Z_bay);
            translate([0, 0, Z_pcb_b - Z_bay]) cylinder(d = 3.0, h = 1.2);
        }
}

module devkit_cradle() {
    h = 3.0;
    translate([dk_x0 - 2, dk_y0 - 2, Z_bay]) cube([2, dk_w + 4, h]);   // 远端挡墙
    translate([dk_x0 - 2, dk_y0 - 2, Z_bay]) cube([dk_l + 2, 2, h]);   // -Y 侧
    translate([dk_x0 - 2, dk_y0 + dk_w, Z_bay]) cube([dk_l + 2, 2, h]);// +Y 侧
}

module pwr_holder() {
    // 母插板竖立，下缘坐在局部凹坑上；两侧夹块 + 点热熔胶
    translate([pwr_x - pwr_wid / 2 - 2.2, pwr_y - 4, Z_bay - 1.0]) cube([2.2, 8, 4.0]);
    translate([pwr_x + pwr_wid / 2, pwr_y - 4, Z_bay - 1.0]) cube([2.2, 8, 4.0]);
}

module wire_clip(p) {
    // 双柱过线梳（无悬空，免支撑）
    translate([p[0] - 4, p[1] - 2, Z_bay]) cube([2.5, 4, 7]);
    translate([p[0] + 1.5, p[1] - 2, Z_bay]) cube([2.5, 4, 7]);
}

module devkit_usb_cut() {
    // 右墙：两个 USB-C 开口
    for (dy = [dk_usb1, dk_usb2])
        translate([bcx + shell_outer / 2 - cav_wall - 0.1,
                   dk_y0 + dy - dk_usb_w / 2 - 1.0,
                   dk_usb_z - dk_usb_h / 2])
            cube([cav_wall + 1.2, dk_usb_w + 2, dk_usb_h]);
    // 盖板凹槽（深 1.5）
    translate([bcx + shell_outer / 2 - 1.5,
               dk_y0 + (dk_usb1 + dk_usb2) / 2 - 17,
               dk_usb_z - 8])
        cube([1.6, 34, 16]);
}

module body() {
    H = Z_sheet_t - Z_bk;
    difference() {
        // ---- 主体 ----
        translate([bcx, bcy, Z_bk])
            linear_extrude(H) rounded_sq(shell_outer, shell_outer, shell_r);

        // ---- 腔体（前缘 = 面板底面 14.8）----
        translate([bcx, bcy, Z_bay])
            linear_extrude(Z_sheet_t - Z_bay + 0.02) rounded_sq(cavity, cavity, 3);

        // ---- 卡扣槽 ----
        translate([bcx, bcy, groove_z - 0.2])
            linear_extrude(groove_h + 0.4) difference() {
                rounded_sq(shell_outer, shell_outer, shell_r);
                rounded_sq(shell_outer - 2 * groove_d,
                           shell_outer - 2 * groove_d, shell_r - 0.6);
            }

        // ---- 背面：USB-C 供电母插 ----
        // 内面局部凹坑（向背板内凹 1.0 → 局部净深 15，母插板 14mm 才放得下）
        translate([pwr_x - pwr_wid / 2 - 1, pwr_y - 6, Z_bay - 1.0])
            cube([pwr_wid + 2, 12, 1.0 + 0.01]);
        // 背板开孔
        translate([pwr_x - pwr_hole_w / 2, pwr_y - pwr_hole_h / 2, Z_bk - 0.1])
            cube([pwr_hole_w, pwr_hole_h, back_t + 1.2]);

        // ---- 右侧墙：DevKit 双 USB-C ----
        devkit_usb_cut();

        // ---- 撬槽（底边外壁 ×2，竖放不可见）----
        for (xx = [60, 132])
            translate([xx - 5, bcy - shell_outer / 2 - 0.1, 3.5])
                cube([10, 1.5, 1.5]);
    }

    pcb_posts();        // 灯板定位柱
    devkit_cradle();    // DevKit 卡座
    pwr_holder();       // 母插夹块
    wire_clip([112, 128]);
    wire_clip([56, 148]);
}

// ============================== USB 盖板 ==============================
// 平放打印（Z 为厚度方向）；外廓 33.6×15.6，对应右墙 34×16 凹槽
module usb_cover() {
    difference() {
        cube([33.6, 15.6, 1.5], center = true);
        translate([0, 0, -0.35]) cube([30.6, 12.6, 1.2], center = true);   // 内侧减薄（留 0.5 面板 + 1.5 边框）
        translate([15.0, 5.0, 0.2]) cube([5, 8, 2], center = true);        // 指甲位
    }
}

// ============================== 3×3 试样 ==============================
// 取灯阵中间 3×3（cols 4…6 / rows 4…6），验证亮度、热斑、横向串光
c0 = 4;  r0 = 4;  cw = 3 * pitch;

module coupon_black() {
    difference() {
        union() {
            translate([0, 0, Z_sheet_t]) linear_extrude(face_t) square([cw + 6, cw + 6]);
            translate([3, 3, 0]) linear_extrude(Z_sheet_b + 0.01) difference() {
                square([cw, cw]);
                for (r = [0:2]) for (c = [0:2])
                    translate([c * pitch + (pitch - grid_cell) / 2,
                               (2 - r) * pitch + (pitch - grid_cell) / 2])
                        square([grid_cell, grid_cell]);
            }
        }
        for (r = [0:2]) for (c = [0:2])
            translate([0, 0, Z_sheet_t - cut_into_m - 0.01])
                linear_extrude(face_t + cut_into_m + 0.02)
                glyph_at(LETTERS[r0 + r][c0 + c],
                         3 + c * pitch + pitch / 2,
                         3 + (2 - r) * pitch + pitch / 2);
    }
}
module coupon_white() {
    difference() {
        translate([3, 3, Z_sheet_b]) linear_extrude(m_t) square([cw, cw]);
        translate([0, 0, Z_sheet_t - cut_into_m - 0.01])
            linear_extrude(cut_into_m + 0.02)
                for (r = [0:2]) for (c = [0:2])
                    glyph_at(LETTERS[r0 + r][c0 + c],
                             3 + c * pitch + pitch / 2,
                             3 + (2 - r) * pitch + pitch / 2);
    }
}
module coupon() { coupon_black(); coupon_white(); }

// ============================== 对齐校验 ==============================
module pcb_ref() {
    color("ForestGreen", 0.35)
        translate([0, 0, Z_pcb_b]) cube([bx, by, bt]);
    color("Gold") for (r = [0:rows - 1]) for (c = [0:cols - 1])
        translate([cell_x(c), cell_y(r), 0]) cube([5, 5, 1.6], center = true);
    color("Red") dot_positions() translate([0, 0, 0])
        cylinder(d = 5, h = 1.6, center = true);
    color("RoyalBlue") {
        for (jx = [j2_x, j1_x])
            translate([jx, hdr_y0 + 1.5 * hdr_dy, hdr_h / 2])
                cube([2.54, 4 * hdr_dy, hdr_h], center = true);
        translate([cap_x, cap_y, cap_h / 2])
            cylinder(d = cap_d, h = cap_h, center = true);
    }
    color("Black") for (p = [[mt_dx, bcy], [bx - mt_dx, bcy],
                             [bcx, mt_dy], [bcx, by - mt_dy]])
        translate([p[0], p[1], Z_pcb_b - 0.5]) cylinder(d = 3.2, h = 4);
    color("DimGray", 0.5)
        translate([dk_x0, dk_y0, Z_bay]) cube([dk_l, dk_w, dk_h]);
    color("Teal", 0.7)
        translate([pwr_x - pwr_wid / 2, pwr_y - 0.8, Z_bay - 1.0])
            cube([pwr_wid, 1.6, pwr_dep]);
}

// ============================== 卡扣试样 ==============================
module snap_test() {
    sg = 90;
    difference() {
        translate([0, -sg / 2, Z_bk]) cube([3, sg, Z_sheet_t - Z_bk]);
        translate([-0.1, -sg / 2 - 0.1, groove_z - 0.1])
            cube([groove_d, sg + 0.2, groove_h + 0.2]);
        translate([-6, -sg / 2 - 0.1, Z_bay]) cube([8, sg + 0.2, 60]);
    }
    translate([12, 0, 0]) {
        translate([-2 * skirt_cl - skirt_t, -sg / 2, skirt_bot])
            cube([skirt_t, sg, Z_sheet_t - skirt_bot + 0.01]);
        translate([-2 * skirt_cl, 0, snap_z]) snap_hook(snap_len, snap_d, snap_h);
    }
}

// ================================ 输出 ================================
if (part == "body")        body();
if (part == "front_black") front_black();
if (part == "front_white") front_white();
if (part == "usb_cover")   usb_cover();
if (part == "coupon")       coupon();          // 仅用于预览着色
if (part == "coupon_black") coupon_black();
if (part == "coupon_white") coupon_white();
if (part == "snap_test")   snap_test();
if (part == "all") {
    color("DimGray")   body();
    color("Gainsboro") front_white();
    color("Black")     front_black();
}
if (part == "check") {
    pcb_ref();
    color("Black", 0.45) front_black();
    color("White", 0.35) front_white();
    color("Gray", 0.18)  body();
}
