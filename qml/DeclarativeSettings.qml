import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Ui as Ui

Flickable {
    id:root
    property var settingsContext
    property var definition: ({})
    property string validationError:""
    contentHeight:form.implicitHeight
    clip:true
    function change(key,value) {
        var next=Object.assign({},settingsContext.draftSettings);next[key]=value;settingsContext.draftSettings=next;
    }
    function city(key,index,field,value) {
        var cities=JSON.parse(JSON.stringify(settingsContext.draftSettings[key] || []));
        cities[index][field]=value;change(key,cities);
    }
    ColumnLayout {
        id:form;width:root.width;spacing:12
        Repeater {
            model:root.definition.settingsUi || []
            ColumnLayout {
                id:field
                required property var modelData
                Layout.fillWidth:true
                readonly property var current:root.settingsContext.draftSettings[modelData.key]
                Text { text:field.modelData.label;textFormat:Text.PlainText;color:Color.foreground;font.bold:true }
                Rectangle {
                    Layout.fillWidth:true;implicitHeight:34
                    visible:field.modelData.type==="text"
                    color:"transparent";border.color:Color.muted
                    TextInput {
                        anchors.fill:parent;anchors.margins:6;color:Color.foreground;clip:true
                        text:typeof field.current==="string"?field.current:""
                        maximumLength:root.definition.settingsSchema.properties[field.modelData.key].maxLength || 128
                        activeFocusOnTab:true;selectByMouse:true
                        onTextEdited:root.change(field.modelData.key,text)
                    }
                }
                Flow {
                    Layout.fillWidth:true;spacing:6
                    visible:field.modelData.type==="choice"
                    Repeater {
                        model:field.modelData.type==="choice"?root.definition.settingsSchema.properties[field.modelData.key].enum:[]
                        Ui.Button { required property string modelData;text:modelData;selected:field.current===modelData;focusable:true;onClicked:root.change(field.modelData.key,modelData) }
                    }
                }
                ColumnLayout {
                    Layout.fillWidth:true
                    visible:field.modelData.type==="timezone-list"
                    Repeater {
                        // Stable integer model preserves the focused input while typing.
                        model:field.modelData.type==="timezone-list" && field.current?field.current.length:0
                        RowLayout {
                            id:cityRow
                            required property int index
                            Layout.fillWidth:true
                            Repeater {
                                model:["label","zone"]
                                Rectangle {
                                    required property string modelData
                                    Layout.fillWidth:true;implicitHeight:34
                                    color:"transparent";border.color:Color.muted
                                    TextInput {
                                        anchors.fill:parent;anchors.margins:6;color:Color.foreground;clip:true
                                        text:field.current[cityRow.index][parent.modelData] || ""
                                        maximumLength:parent.modelData==="label"?80:128
                                        activeFocusOnTab:true;selectByMouse:true
                                        onTextEdited:root.city(field.modelData.key,cityRow.index,parent.modelData,text)
                                    }
                                }
                            }
                            Ui.Button { text:"Remove";focusable:true;onClicked:{var a=field.current.slice();a.splice(cityRow.index,1);root.change(field.modelData.key,a);} }
                        }
                    }
                    Ui.Button {
                        text:"Add city";focusable:true
                        enabled:!field.current || field.current.length<root.definition.settingsSchema.properties[field.modelData.key].maxItems
                        onClicked:root.change(field.modelData.key,(field.current || []).concat([{label:"UTC",zone:"Etc/UTC"}]))
                    }
                }
            }
        }
    }
}
