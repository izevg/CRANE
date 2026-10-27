{{- define "crane.fullname" -}}
{{- printf "%s-%s" .Release.Name .Chart.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "crane.image" -}}
{{- printf "%s:%s" .Values.image.repository (default .Chart.AppVersion .Values.image.tag) -}}
{{- end -}}

{{- define "crane.labels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version }}
{{- end -}}

{{- define "crane.selector" -}}
app.kubernetes.io/name: {{ .root.Chart.Name }}
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .component }}
{{- end -}}

{{/* Общая часть пода: образ, окружение и жёсткий securityContext (restricted PSS). */}}
{{- define "crane.podspec" -}}
{{- with .root.Values.imagePullSecrets }}
imagePullSecrets: {{ toYaml . | nindent 2 }}
{{- end }}
securityContext:
  runAsNonRoot: true
  seccompProfile: { type: RuntimeDefault }
containers:
  - name: {{ .component }}
    image: {{ include "crane.image" .root }}
    imagePullPolicy: {{ .root.Values.image.pullPolicy }}
    args: [{{ .role | quote }}]
    envFrom:
      - secretRef:
          name: {{ required "existingSecret обязателен" .root.Values.existingSecret }}
    env:
      - name: CRANE_CACHE
        value: {{ .root.Values.config.cache | quote }}
      {{- with .root.Values.config.blobs }}
      - name: CRANE_BLOBS
        value: {{ . | quote }}
      {{- end }}
    securityContext:
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities: { drop: [ALL] }
    volumeMounts:
      - { name: tmp, mountPath: /tmp }
volumes:
  - { name: tmp, emptyDir: {} }
{{- end -}}
