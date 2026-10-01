// A theme sets every paint the renderer reads (SPEC section 13.4): surfaces,
// inks, the frame and eight container tones, the tag, fact and ask boxes, the
// four lines, drawn widths, the icon chip, isometric shading and the eight tint
// slots. It knows roles and tones, never a grammar's kinds. The Rust model
// (crates/stencil-model/src/theme.rs) is the twin of this definition; where the
// two disagree the Rust types win. A theme file vets with
//   cue vet -c -d '#Theme' . <file>.json
package stencil

import "list"

// Uppercase #RRGGBB.
#Color:   =~"^#[0-9A-F]{6}$"
#Pattern: "solid" | "dashed" | "dotted"
#Dot:     "filled" | "hollow" | "none"

// A drawn stroke width in px. Layout reserves the grammar's widths whatever is drawn.
#Width: number & >=0.5 & <=4

// HSL lightness step of an isometric face, in percentage points.
#Step:    int & >=-40 & <=40
#Opacity: number & >=0 & <=1

#Stroke: {
	color:   #Color
	width:   #Width
	pattern: #Pattern
}

#Swatch: {
	fill: #Color
	ink:  #Color
}

#Accent: {
	accent: #Color
	fill:   #Color
}

#ToneRole: {
	fill?:      #Color
	border?:    #Color
	label_ink?: #Color
}

#Tint: {
	name:   =~"^[a-z]{1,12}$"
	fill:   #Color
	border: #Color
	ink:    #Color
	wire:   #Color
}

#Theme: {
	name: =~"^[a-z][a-z0-9-]{0,31}$"
	// color: tints are told apart by color, legend labels name the color, and the
	// separation thresholds apply. line: tints are told apart by end dots and pattern,
	// legend labels name the line, and the separation thresholds do not apply.
	tint_cue: "color" | "line"
	page:     #Color
	ink: {
		primary:    #Color // title, item title, block body, list bullets
		secondary:  #Color // lede, item subtitle
		zone_label: #Color // an untinted container's label, a Frame block's label
	}
	kicker: #Color
	badge: {
		customer: #Swatch
		internal: #Swatch
		border?:  #Stroke
	}
	card: {
		fill:   #Color
		border: #Stroke
	}
	fact: #Swatch
	ask:  #Swatch
	tag: {
		fill:    #Color
		border:  #Stroke
		ink:     #Color
		sub_ink: #Color
	}
	legend: {
		label_ink: #Color
		text_ink:  #Color // legend text and Note kind legend
	}
	foot: #Color
	callout: {
		note:     #Accent
		risk:     #Accent
		decision: #Accent
		open:     #Accent
	}
	// The Frame block of section 11.3, a wireframe placeholder.
	placeholder: {
		border:   #Color
		diagonal: #Color
	}
	// The flat chip under every icon; absent draws none.
	icon_chip?: #Color
	// The frame role.
	frame: {
		border:     #Color
		frame_fill: #Color
		bar_fill:   #Color
		bar_ink:    #Color
		bar_rule?:  #Stroke
		body_fill:  #Color
	}
	tones: {
		neutral: #ToneRole
		warm:    #ToneRole
		cool:    #ToneRole
		soft:    #ToneRole
		// strong never fills: a container of this tone is a ring under iso.
		strong: {
			border?:    #Color
			label_ink?: #Color
		}
		highlight: #ToneRole
		emphasis:  #ToneRole
		accent:    #ToneRole
	}
	containers: {
		// Draw every container border but a frame's at this width; absent draws the
		// grammar's width.
		draw_width?:       #Width
		frame_draw_width?: #Width
		// A border for containers whose grammar pattern is none; absent draws none.
		borderless_outline?: #Stroke
	}
	lanes: lifeline: #Stroke
	gray: {
		color:   #Color
		width:   #Width
		pattern: #Pattern
		dot:     #Dot
	}
	// Solid and dash lines take their color from the tint slot's wire.
	solid: {
		width: #Width
		dots: [...#Dot] & list.MinItems(8) & list.MaxItems(8)
	}
	dash: {
		width:   #Width
		pattern: #Pattern
		dot:     #Dot
	}
	deny: {
		color:      #Color
		width:      #Width
		pattern:    #Pattern
		dot:        #Dot
		tag_border: #Color
		tag_ink:    #Color
	}
	tints: [...#Tint] & list.MinItems(8) & list.MaxItems(8)
	iso: {
		faces: {
			top:   #Step
			left:  #Step
			right: #Step
		}
		slab_thickness: number & >=2 & <=16
		// The frame slab's side faces; absent shades the frame body fill like any slab.
		frame_sides?: {
			left:  #Color
			right: #Color
		}
		frame_outline: #Width
		// none: a solid container border is not drawn on the slab, the shaded sides carry
		// the edge. outline: drawn at edge_width on the top face and the sides.
		solid_edges: "none" | "outline"
		edge_width:  #Width
		// A ring (a container with no drawn fill); absent draws its flat border.
		ring?: #Stroke
		// The outline of a block whose flat drawing has no border; absent draws none.
		block_outline?: #Stroke
		plates:         bool
		tabs: {
			frame:        #Swatch
			top:          #Swatch
			nested_width: #Width
		}
		chip: {
			fill:  #Color
			ring?: #Color
			shadow?: {
				color:   #Color
				opacity: #Opacity
				dy:      number & >=0 & <=4
			}
		}
		// A soft shadow under every opaque block (section 13.11); absent draws none.
		shadow?: {
			color:   #Color
			opacity: #Opacity
			blur:    number & >0 & <=8
			dy:      number & >=0 & <=8
		}
		widths: {
			primary:   #Width // solid slot 1
			secondary: #Width // solid slots 2 to 8, dash, deny
			gray:      #Width
		}
	}
}
